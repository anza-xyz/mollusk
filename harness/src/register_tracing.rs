use {
    crate::{
        register_tracing_filter::{eval, expr},
        InvocationInspectCallback, Mollusk,
    },
    sha2::{Digest, Sha256},
    solana_program_runtime::invoke_context::{Executable, InvokeContext, RegisterTrace},
    solana_pubkey::Pubkey,
    solana_transaction_context::{InstructionAccount, InstructionContext},
    std::{collections::HashMap, fs::File, io::Write},
};

const DEFAULT_PATH: &str = "target/sbf/trace";
#[cfg(feature = "sbpf-debugger")]
const DEFAULT_DEBUG_PORT: Option<u16> = None;

pub struct DefaultRegisterTracingCallback {
    pub sbf_trace_dir: String,
    pub sbf_trace_disassemble: bool,
    pub sbf_trace_filter: String,
    #[cfg(feature = "sbpf-debugger")]
    pub sbf_debug_port: Option<u16>,
}

impl Default for DefaultRegisterTracingCallback {
    fn default() -> Self {
        Self {
            // User can override default path with `SBF_TRACE_DIR` environment variable.
            sbf_trace_dir: std::env::var("SBF_TRACE_DIR").unwrap_or(DEFAULT_PATH.to_string()),
            sbf_trace_disassemble: std::env::var("SBF_TRACE_DISASSEMBLE").is_ok(),
            sbf_trace_filter: std::env::var("SBF_TRACE_FILTER").unwrap_or_default(),
            // The port that will be used for debugging.
            // Will invoke the debugger if set.
            #[cfg(feature = "sbpf-debugger")]
            sbf_debug_port: std::env::var("SBF_DEBUG_PORT")
                .map(|port| port.parse::<u16>().ok())
                .unwrap_or(DEFAULT_DEBUG_PORT),
        }
    }
}

impl DefaultRegisterTracingCallback {
    pub fn disassemble_register_trace<W: std::io::Write>(
        &self,
        writer: &mut W,
        program_id: &Pubkey,
        executable: &Executable,
        register_trace: RegisterTrace,
    ) {
        match solana_program_runtime::solana_sbpf::static_analysis::Analysis::from_executable(
            executable,
        ) {
            Ok(analysis) => {
                if let Err(e) = analysis.disassemble_register_trace(writer, register_trace) {
                    eprintln!("Can't disassemble register trace for {program_id}: {e:#?}");
                }
            }
            Err(e) => {
                eprintln!("Can't create trace disassemble analysis for {program_id}: {e:#?}")
            }
        }
    }

    pub fn match_filter(&self, program_ids: Vec<String>) -> bool {
        let Ok((_, ast)) = expr(&self.sbf_trace_filter) else {
            return true;
        };
        let row = HashMap::from([("program_id", program_ids)]);
        eval(&ast, &row)
    }

    pub fn pre_handler(
        &self,
        mollusk: &Mollusk,
        program_id: &Pubkey,
        _instruction_data: &[u8],
        instruction_accounts: &[InstructionAccount],
        invoke_context: &mut InvokeContext,
    ) {
        #[cfg(feature = "sbpf-debugger")]
        {
            // Persist SHA-256 mapping for every ELF account.
            // We need them later to judge what symbol object to
            // load in the debugger client.
            if let Ok(program_ids) = self.elf_accounts_to_sha256(
                mollusk,
                program_id,
                instruction_accounts,
                invoke_context,
            ) {
                // Any program id that matches the filter should invoke the debugger.
                let program_ids = program_ids
                    .iter()
                    .map(|program_id| program_id.to_string())
                    .collect();
                if let Some(debug_port) = self.sbf_debug_port {
                    if self.match_filter(program_ids) {
                        invoke_context.debug_port = Some(debug_port);
                    }
                }
            }
        }
    }

    pub fn post_handler(
        &self,
        mollusk: &Mollusk,
        instruction_context: InstructionContext,
        executable: &Executable,
        register_trace: RegisterTrace,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if register_trace.is_empty() {
            // Can't do much with an empty trace.
            return Ok(());
        }

        // Get program_id.
        let program_id = instruction_context.get_program_key()?;
        if !self.match_filter(vec![program_id.to_string()]) {
            // Skip this one since no filter has matched.
            return Ok(());
        }

        let current_dir = std::env::current_dir()?;
        let sbf_trace_dir = current_dir.join(&self.sbf_trace_dir);
        std::fs::create_dir_all(&sbf_trace_dir)?;

        let trace_digest = compute_hash(as_bytes(register_trace));
        let base_fname = sbf_trace_dir.join(&trace_digest[..16]);
        let mut regs_file = File::create(base_fname.with_extension("regs"))?;
        let mut insns_file = File::create(base_fname.with_extension("insns"))?;
        let mut program_id_file = File::create(base_fname.with_extension("program_id"))?;

        // Persist a full trace disassembly if requested.
        if self.sbf_trace_disassemble {
            let mut trace_disassemble_file = File::create(base_fname.with_extension("trace"))?;
            self.disassemble_register_trace(
                &mut trace_disassemble_file,
                program_id,
                executable,
                register_trace,
            );
        }

        // Persist the program id.
        let _ = program_id_file.write(program_id.to_string().as_bytes());

        if let Some(elf_data) = mollusk.program_cache.get_program_elf_bytes(program_id) {
            // Persist the preload hash of the executable.
            let mut so_hash_file = File::create(base_fname.with_extension("exec.sha256"))?;
            let _ = so_hash_file.write(compute_hash(elf_data.as_slice()).as_bytes());
        }

        // Get the relocated executable.
        let (_, program) = executable.get_text_bytes();
        for regs in register_trace.iter() {
            // The program counter is stored in r11.
            let pc = regs[11];
            // From the executable fetch the instruction this program counter points to.
            let insn =
                solana_program_runtime::solana_sbpf::ebpf::get_insn_unchecked(program, pc as usize)
                    .to_array();

            // Persist them in files.
            let _ = regs_file.write(as_bytes(regs.as_slice()))?;
            let _ = insns_file.write(insn.as_slice())?;
        }

        Ok(())
    }

    /// Persists a mapping of program IDs to SHA-256 hashes of their ELF bytes.
    /// Includes the top-level program and any instruction accounts that are
    /// programs in the cache. This allows a debugger client to resolve which
    /// .so to load for symbol information. Returns the list of program IDs
    /// for which an ELF was found in the cache.
    pub fn elf_accounts_to_sha256(
        &self,
        mollusk: &Mollusk,
        program_id: &Pubkey,
        instruction_accounts: &[InstructionAccount],
        invoke_context: &InvokeContext,
    ) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
        let current_dir = std::env::current_dir()?;
        let sbf_trace_dir = current_dir.join(&self.sbf_trace_dir);
        std::fs::create_dir_all(&sbf_trace_dir)?;
        let base_fname = sbf_trace_dir.join("program_ids");
        let mut program_ids = Vec::new();
        let mut program_ids_file = File::create(base_fname.with_extension("map"))?;

        let mut persist_elf_sha256 = |file: &mut File, pubkey: &Pubkey| {
            if let Some(elf_data) = mollusk.program_cache.get_program_elf_bytes(pubkey) {
                program_ids.push(*pubkey);
                let _ = file.write(
                    format!("{}={}\n", pubkey, compute_hash(elf_data.as_slice())).as_bytes(),
                );
            }
        };

        persist_elf_sha256(&mut program_ids_file, program_id);

        instruction_accounts
            .iter()
            .flat_map(|ia| {
                invoke_context
                    .transaction_context
                    .get_key_of_account_at_index(ia.index_in_transaction)
            })
            .for_each(|pubkey| {
                persist_elf_sha256(&mut program_ids_file, pubkey);
            });

        Ok(program_ids)
    }
}

impl InvocationInspectCallback for DefaultRegisterTracingCallback {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn before_invocation(
        &self,
        mollusk: &Mollusk,
        program_id: &Pubkey,
        instruction_data: &[u8],
        instruction_accounts: &[InstructionAccount],
        invoke_context: &mut InvokeContext,
        register_tracing_enabled: bool,
    ) {
        if register_tracing_enabled {
            self.pre_handler(
                mollusk,
                program_id,
                instruction_data,
                instruction_accounts,
                invoke_context,
            );
        }
    }

    fn after_invocation(
        &self,
        mollusk: &Mollusk,
        invoke_context: &InvokeContext,
        register_tracing_enabled: bool,
    ) {
        if register_tracing_enabled {
            // Only read the register traces if they were actually enabled.
            invoke_context.iterate_vm_traces(
                &|instruction_context: InstructionContext,
                  executable: &Executable,
                  register_trace: RegisterTrace| {
                    if let Err(e) =
                        self.post_handler(mollusk, instruction_context, executable, register_trace)
                    {
                        eprintln!("Error collecting the register tracing: {}", e);
                    }
                },
            );
        }
    }
}

pub(crate) fn as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

pub fn compute_hash(slice: &[u8]) -> String {
    hex::encode(Sha256::digest(slice).as_slice())
}
