use {
    crate::InvocationInspectCallback,
    sha2::{Digest, Sha256},
    solana_program_runtime::invoke_context::{Executable, InvokeContext, RegisterTrace},
    solana_pubkey::Pubkey,
    solana_transaction_context::{InstructionAccount, InstructionContext},
    std::io::Write,
};

pub struct DefaultRegisterTracingCallback;

impl InvocationInspectCallback for DefaultRegisterTracingCallback {
    fn before_invocation(&self, _: &Pubkey, _: &[u8], _: &[InstructionAccount], _: &InvokeContext) {
    }

    fn after_invocation(&self, invoke_context: &InvokeContext) {
        invoke_context.iterate_vm_traces(
            &|instruction_context: InstructionContext,
              executable: &Executable,
              register_trace: RegisterTrace| {
                if let Err(e) = default_register_tracing_callback(
                    instruction_context,
                    executable,
                    register_trace,
                ) {
                    eprintln!("Error collecting the register tracing: {}", e);
                }
            },
        );
    }
}

pub fn default_register_tracing_callback(
    _instruction_context: InstructionContext,
    executable: &Executable,
    register_trace: RegisterTrace,
) -> Result<(), Box<dyn std::error::Error>> {
    if register_trace.is_empty() {
        // Can't do much with an empty trace.
        return Ok(());
    }

    if let Ok(sbf_trace_dir) = &std::env::var("SBF_TRACE_DIR") {
        let current_dir = std::env::current_dir()?;
        let sbf_trace_dir = current_dir.join(sbf_trace_dir);
        std::fs::create_dir_all(&sbf_trace_dir)?;

        let digest = Sha256::digest(as_bytes(register_trace));
        let base_fname = sbf_trace_dir.join(&hex::encode(digest)[..16]);
        let mut regs_file = std::fs::File::create(base_fname.with_extension("regs"))?;
        let mut insns_file = std::fs::File::create(base_fname.with_extension("insns"))?;

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
    }

    Ok(())
}

pub(crate) fn as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}
