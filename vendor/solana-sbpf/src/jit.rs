//! Just-in-time compiler (Linux x86, macOS x86)

// Derived from uBPF <https://github.com/iovisor/ubpf>
// Copyright 2015 Big Switch Networks, Inc
//      (uBPF: JIT algorithm, originally in C)
// Copyright 2016 6WIND S.A. <quentin.monnet@6wind.com>
//      (Translation to Rust, MetaBuff addition)
// Copyright 2020 Solana Maintainers <maintainers@solana.com>
//
// Licensed under the Apache License, Version 2.0 <http://www.apache.org/licenses/LICENSE-2.0> or
// the MIT license <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(clippy::arithmetic_side_effects)]

#[cfg(not(feature = "shuttle-test"))]
use rand::{thread_rng, Rng};
#[cfg(feature = "shuttle-test")]
use shuttle::rand::{thread_rng, Rng};

use rand::{
    distributions::{Distribution, Uniform},
    rngs::SmallRng,
    SeedableRng,
};
use std::{fmt::Debug, mem, ptr};

use crate::{
    ebpf::{self, FIRST_SCRATCH_REG, FRAME_PTR_REG, INSN_SIZE, SCRATCH_REGS},
    elf::Executable,
    error::{EbpfError, ProgramResult},
    memory_management::{
        allocate_pages, free_pages, get_system_page_size, protect_pages, round_to_page_size,
    },
    memory_region::MemoryMapping,
    program::BuiltinFunction,
    vm::{get_runtime_environment_key, Config, ContextObject, EbpfVm, RuntimeEnvironmentSlot},
    x86::{
        FenceType, X86IndirectAccess, X86Instruction,
        X86Register::{self, *},
        ARGUMENT_REGISTERS, CALLEE_SAVED_REGISTERS, CALLER_SAVED_REGISTERS,
    },
};

/// The maximum machine code length in bytes of a program with no guest instructions
pub const MAX_EMPTY_PROGRAM_MACHINE_CODE_LENGTH: usize = 4096;
/// The maximum machine code length in bytes of a single guest instruction
pub const MAX_MACHINE_CODE_LENGTH_PER_INSTRUCTION: usize = 110;
/// The maximum machine code length in bytes of an instruction meter checkpoint
pub const MACHINE_CODE_PER_INSTRUCTION_METER_CHECKPOINT: usize = 24;
/// The maximum machine code length of the randomized padding
pub const MAX_START_PADDING_LENGTH: usize = 256;

/// The program compiled to native host machinecode
pub struct JitProgram {
    /// OS page size in bytes and the alignment of the sections
    page_size: usize,
    /// Byte offset in the text_section for each BPF instruction
    pc_section: &'static mut [u32],
    /// The x86 machinecode
    text_section: &'static mut [u8],
}

impl JitProgram {
    fn new(pc: usize, code_size: usize) -> Result<Self, EbpfError> {
        let page_size = get_system_page_size();
        let pc_loc_table_size = round_to_page_size(pc * std::mem::size_of::<u32>(), page_size);
        let over_allocated_code_size = round_to_page_size(code_size, page_size);
        unsafe {
            let raw = allocate_pages(pc_loc_table_size + over_allocated_code_size)?;
            Ok(Self {
                page_size,
                pc_section: std::slice::from_raw_parts_mut(raw.cast::<u32>(), pc),
                text_section: std::slice::from_raw_parts_mut(
                    raw.add(pc_loc_table_size),
                    over_allocated_code_size,
                ),
            })
        }
    }

    fn seal(&mut self, text_section_usage: usize) -> Result<(), EbpfError> {
        if self.page_size == 0 {
            return Ok(());
        }
        let raw = self.pc_section.as_ptr() as *mut u8;
        let pc_loc_table_size =
            round_to_page_size(std::mem::size_of_val(self.pc_section), self.page_size);
        let over_allocated_code_size = round_to_page_size(self.text_section.len(), self.page_size);
        let code_size = round_to_page_size(text_section_usage, self.page_size);
        unsafe {
            // Fill with debugger traps
            std::ptr::write_bytes(
                raw.add(pc_loc_table_size).add(text_section_usage),
                0xcc,
                code_size - text_section_usage,
            );
            if over_allocated_code_size > code_size {
                free_pages(
                    raw.add(pc_loc_table_size).add(code_size),
                    over_allocated_code_size - code_size,
                )?;
            }
            self.text_section =
                std::slice::from_raw_parts_mut(raw.add(pc_loc_table_size), text_section_usage);
            protect_pages(
                self.pc_section.as_mut_ptr().cast::<u8>(),
                pc_loc_table_size,
                false,
            )?;
            protect_pages(self.text_section.as_mut_ptr(), code_size, true)?;
        }
        Ok(())
    }

    pub(crate) fn invoke<C: ContextObject>(
        &self,
        _config: &Config,
        vm: &mut EbpfVm<C>,
        registers: [u64; 12],
    ) {
        unsafe {
            let runtime_environment = std::ptr::addr_of_mut!(*vm)
                .cast::<u64>()
                .offset(get_runtime_environment_key() as isize);
            let instruction_meter =
                (vm.previous_instruction_meter as i64).wrapping_add(registers[11] as i64);
            let entrypoint = &self.text_section
                [self.pc_section[registers[11] as usize] as usize & (i32::MAX as u32 as usize)]
                as *const u8;
            macro_rules! stmt_expr_attribute_asm {
                ($($prologue:literal,)+ cfg(not(feature = $feature:literal)), $guarded:tt, $($epilogue:tt)+) => {
                    #[cfg(feature = $feature)]
                    std::arch::asm!($($prologue,)+ $($epilogue)+);
                    #[cfg(not(feature = $feature))]
                    std::arch::asm!($($prologue,)+ $guarded, $($epilogue)+);
                }
            }
            stmt_expr_attribute_asm!(
                // RBP and RBX must be saved and restored manually in the current version of rustc and llvm.
                "push rbx",
                "push rbp",
                "mov [{host_stack_pointer}], rsp",
                "add QWORD PTR [{host_stack_pointer}], -8",
                // RBP is zeroed out in order not to compromise the runtime environment (RDI) encryption.
                cfg(not(feature = "jit-enable-host-stack-frames")),
                "xor rbp, rbp",
                "mov [rsp-8], rax",
                "mov rax, [r11 + 0x00]",
                "mov rsi, [r11 + 0x08]",
                "mov rdx, [r11 + 0x10]",
                "mov rcx, [r11 + 0x18]",
                "mov r8,  [r11 + 0x20]",
                "mov r9,  [r11 + 0x28]",
                "mov rbx, [r11 + 0x30]",
                "mov r12, [r11 + 0x38]",
                "mov r13, [r11 + 0x40]",
                "mov r14, [r11 + 0x48]",
                "mov r15, [r11 + 0x50]",
                "mov r11, [r11 + 0x58]",
                "call [rsp-8]",
                "pop rbp",
                "pop rbx",
                host_stack_pointer = in(reg) &mut vm.host_stack_pointer,
                inlateout("rdi") runtime_environment => _,
                inlateout("r10") instruction_meter => _,
                inlateout("rax") entrypoint => _,
                inlateout("r11") &registers => _,
                lateout("rsi") _, lateout("rdx") _, lateout("rcx") _, lateout("r8") _,
                lateout("r9") _, lateout("r12") _, lateout("r13") _, lateout("r14") _, lateout("r15") _,
                // lateout("rbp") _, lateout("rbx") _,
            );
        }
    }

    /// The length of the host machinecode in bytes
    pub fn machine_code_length(&self) -> usize {
        self.text_section.len()
    }

    /// The total memory used in bytes rounded up to page boundaries
    pub fn mem_size(&self) -> usize {
        let pc_loc_table_size =
            round_to_page_size(std::mem::size_of_val(self.pc_section), self.page_size);
        let code_size = round_to_page_size(self.text_section.len(), self.page_size);
        pc_loc_table_size + code_size
    }
}

impl Drop for JitProgram {
    fn drop(&mut self) {
        let pc_loc_table_size =
            round_to_page_size(std::mem::size_of_val(self.pc_section), self.page_size);
        let code_size = round_to_page_size(self.text_section.len(), self.page_size);
        if pc_loc_table_size + code_size > 0 {
            unsafe {
                let _ = free_pages(
                    self.pc_section.as_ptr() as *mut u8,
                    pc_loc_table_size + code_size,
                );
            }
        }
    }
}

impl Debug for JitProgram {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_fmt(format_args!("JitProgram {:?}", self as *const _))
    }
}

impl PartialEq for JitProgram {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self as *const _, other as *const _)
    }
}

// Used to define subroutines and then call them
// See JitCompiler::set_anchor() and JitCompiler::relative_to_anchor()
const ANCHOR_TRACE: usize = 0;
const ANCHOR_THROW_EXCEEDED_MAX_INSTRUCTIONS: usize = 1;
const ANCHOR_EPILOGUE: usize = 2;
const ANCHOR_THROW_EXCEPTION_UNCHECKED: usize = 3;
const ANCHOR_EXIT: usize = 4;
const ANCHOR_THROW_EXCEPTION: usize = 5;
const ANCHOR_CALL_DEPTH_EXCEEDED: usize = 6;
const ANCHOR_CALL_OUTSIDE_TEXT_SEGMENT: usize = 7;
const ANCHOR_DIV_BY_ZERO: usize = 8;
const ANCHOR_DIV_OVERFLOW: usize = 9;
const ANCHOR_CALL_REG_UNSUPPORTED_INSTRUCTION: usize = 10;
const ANCHOR_CALL_UNSUPPORTED_INSTRUCTION: usize = 11;
const ANCHOR_EXTERNAL_FUNCTION_CALL: usize = 12;
const ANCHOR_INTERNAL_FUNCTION_CALL_PROLOGUE: usize = 13;
const ANCHOR_INTERNAL_FUNCTION_CALL_REG: usize = 14;
const ANCHOR_TRANSLATE_MEMORY_ADDRESS: usize = 21;
const ANCHOR_COUNT: usize = 34; // Update me when adding or removing anchors

const REGISTER_MAP: [X86Register; 11] = [
    CALLER_SAVED_REGISTERS[0], // RAX
    ARGUMENT_REGISTERS[1],     // RSI
    ARGUMENT_REGISTERS[2],     // RDX
    ARGUMENT_REGISTERS[3],     // RCX
    ARGUMENT_REGISTERS[4],     // R8
    ARGUMENT_REGISTERS[5],     // R9
    CALLEE_SAVED_REGISTERS[1], // RBX
    CALLEE_SAVED_REGISTERS[2], // R12
    CALLEE_SAVED_REGISTERS[3], // R13
    CALLEE_SAVED_REGISTERS[4], // R14
    CALLEE_SAVED_REGISTERS[5], // R15
];

/// RDI: Used together with slot_in_vm()
const REGISTER_PTR_TO_VM: X86Register = ARGUMENT_REGISTERS[0];
/// R10: Program counter limit
const REGISTER_INSTRUCTION_METER: X86Register = CALLER_SAVED_REGISTERS[7];
/// R11: Scratch register
const REGISTER_SCRATCH: X86Register = CALLER_SAVED_REGISTERS[8];

/// Bit width of an instruction operand
#[derive(Copy, Clone, Debug)]
pub enum OperandSize {
    /// Empty
    S0 = 0,
    /// 8 bit
    S8 = 8,
    /// 16 bit
    S16 = 16,
    /// 32 bit
    S32 = 32,
    /// 64 bit
    S64 = 64,
}

enum Value {
    Register(X86Register),
    RegisterIndirect(X86Register, i32, bool),
    RegisterPlusConstant32(X86Register, i32, bool),
    RegisterPlusConstant64(X86Register, i64, bool),
    Constant64(i64, bool),
}

struct Argument {
    index: usize,
    value: Value,
}

#[derive(Debug)]
struct Jump {
    location: *const u8,
    target_pc: usize,
}

/* Explanation of the Instruction Meter

    The instruction meter serves two purposes: First, measure how many BPF instructions are
    executed (profiling) and second, limit this number by stopping the program with an exception
    once a given threshold is reached (validation). One approach would be to increment and
    validate the instruction meter before each instruction. However, this would heavily impact
    performance. Thus, we only profile and validate the instruction meter at branches.

    For this, we implicitly sum up all the instructions between two branches.
    It is easy to know the end of such a slice of instructions, but how do we know where it
    started? There could be multiple ways to jump onto a path which all lead to the same final
    branch. This is, where the integral technique comes in. The program is basically a sequence
    of instructions with the x-axis being the program counter (short "pc"). The cost function is
    a constant function which returns one for every point on the x axis. Now, the instruction
    meter needs to calculate the definite integral of the cost function between the start and the
    end of the current slice of instructions. For that we need the indefinite integral of the cost
    function. Fortunately, the derivative of the pc is the cost function (it increases by one for
    every instruction), thus the pc is an antiderivative of the the cost function and a valid
    indefinite integral. So, to calculate an definite integral of the cost function, we just need
    to subtract the start pc from the end pc of the slice. This difference can then be subtracted
    from the remaining instruction counter until it goes below zero at which point it reaches
    the instruction meter limit. Ok, but how do we know the start of the slice at the end?

    The trick is: We do not need to know. As subtraction and addition are associative operations,
    we can reorder them, even beyond the current branch. Thus, we can simply account for the
    amount the start will subtract at the next branch by already adding that to the remaining
    instruction counter at the current branch. So, every branch just subtracts its current pc
    (the end of the slice) and adds the target pc (the start of the next slice) to the remaining
    instruction counter. This way, no branch needs to know the pc of the last branch explicitly.
    Another way to think about this trick is as follows: The remaining instruction counter now
    measures what the maximum pc is, that we can reach with the remaining budget after the last
    branch.

    One problem are conditional branches. There are basically two ways to handle them: Either,
    only do the profiling if the branch is taken, which requires two jumps (one for the profiling
    and one to get to the target pc). Or, always profile it as if the jump to the target pc was
    taken, but then behind the conditional branch, undo the profiling (as it was not taken). We
    use the second method and the undo profiling is the same as the normal profiling, just with
    reversed plus and minus signs.

    Another special case to keep in mind are return instructions. They would require us to know
    the return address (target pc), but in the JIT we already converted that to be a host address.
    Of course, one could also save the BPF return address on the stack, but an even simpler
    solution exists: Just count as if you were jumping to an specific target pc before the exit,
    and then after returning use the undo profiling. The trick is, that the undo profiling now
    has the current pc which is the BPF return address. The virtual target pc we count towards
    and undo again can be anything, so we just set it to zero.
*/

/// Temporary object which stores the compilation context
pub struct JitCompiler<'a, C: ContextObject> {
    result: JitProgram,
    text_section_jumps: Vec<Jump>,
    anchors: [*const u8; ANCHOR_COUNT],
    offset_in_text_section: usize,
    executable: &'a Executable<C>,
    program: &'a [u8],
    program_vm_addr: u64,
    config: &'a Config,
    pc: usize,
    last_instruction_meter_validation_pc: usize,
    next_noop_insertion: u32,
    noop_range: Uniform<u32>,
    runtime_environment_key: i32,
    immediate_value_key: i64,
    diversification_rng: SmallRng,
    stopwatch_is_active: bool,
}

#[rustfmt::skip]
impl<'a, C: ContextObject> JitCompiler<'a, C> {
    /// Constructs a new compiler and allocates memory for the compilation output
    pub fn new(executable: &'a Executable<C>) -> Result<Self, EbpfError> {
        let config = executable.get_config();
        let (program_vm_addr, program) = executable.get_text_bytes();

        // Scan through program to find actual number of instructions
        let mut pc = 0;
        if !executable.get_sbpf_version().disable_lddw() {
            while (pc + 1) * ebpf::INSN_SIZE <= program.len() {
                let insn = ebpf::get_insn_unchecked(program, pc);
                pc += match insn.opc {
                    ebpf::LD_DW_IMM => 2,
                    _ => 1,
                };
            }
        } else {
            pc = program.len() / ebpf::INSN_SIZE;
        }

        let mut code_length_estimate = MAX_EMPTY_PROGRAM_MACHINE_CODE_LENGTH + MAX_START_PADDING_LENGTH + MAX_MACHINE_CODE_LENGTH_PER_INSTRUCTION * pc;
        if config.noop_instruction_rate != 0 {
            code_length_estimate += code_length_estimate / config.noop_instruction_rate as usize;
        }
        if config.instruction_meter_checkpoint_distance != 0 {
            code_length_estimate += pc / config.instruction_meter_checkpoint_distance * MACHINE_CODE_PER_INSTRUCTION_METER_CHECKPOINT;
        }
        // Relative jump destinations limit the maximum output size
        debug_assert!(code_length_estimate < (i32::MAX as usize));

        let runtime_environment_key = get_runtime_environment_key();
        let mut diversification_rng = SmallRng::from_rng(thread_rng()).map_err(|_| EbpfError::JitNotCompiled)?;
        let immediate_value_key = diversification_rng.gen::<i64>();

        Ok(Self {
            result: JitProgram::new(pc, code_length_estimate)?,
            text_section_jumps: vec![],
            anchors: [std::ptr::null(); ANCHOR_COUNT],
            offset_in_text_section: 0,
            executable,
            program_vm_addr,
            program,
            config,
            pc: 0,
            last_instruction_meter_validation_pc: 0,
            next_noop_insertion: if config.noop_instruction_rate == 0 { u32::MAX } else { diversification_rng.gen_range(0..config.noop_instruction_rate * 2) },
            noop_range: Uniform::new_inclusive(0, config.noop_instruction_rate * 2),
            runtime_environment_key,
            immediate_value_key,
            diversification_rng,
            stopwatch_is_active: false,
        })
    }

    /// Compiles the given executable, consuming the compiler
    pub fn compile(mut self) -> Result<JitProgram, EbpfError> {
        // Randomized padding at the start before random intervals begin
        if self.config.noop_instruction_rate != 0 {
            for _ in 0..self.diversification_rng.gen_range(0..MAX_START_PADDING_LENGTH) {
                // X86Instruction::noop().emit(self)?;
                self.emit::<u8>(0x90);
            }
        }

        self.emit_subroutines();

        while self.pc * ebpf::INSN_SIZE < self.program.len() {
            if self.offset_in_text_section + MAX_MACHINE_CODE_LENGTH_PER_INSTRUCTION * 2 >= self.result.text_section.len() {
                return Err(EbpfError::ExhaustedTextSegment(self.pc));
            }
            let mut insn = ebpf::get_insn_unchecked(self.program, self.pc);
            self.result.pc_section[self.pc] = self.offset_in_text_section as u32;
            if self.executable.get_sbpf_version().enable_stricter_verification() &&
               !insn.is_function_start_marker() {
                self.result.pc_section[self.pc] |= 1 << 31;
            }

            // Regular instruction meter checkpoints to prevent long linear runs from exceeding their budget
            if self.last_instruction_meter_validation_pc + self.config.instruction_meter_checkpoint_distance <= self.pc {
                self.emit_validate_instruction_count(Some(self.pc));
            }

            if self.config.enable_register_tracing {
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.pc as i64));
                self.emit_ins(X86Instruction::call_immediate(self.relative_to_anchor(ANCHOR_TRACE, 5)));
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, 0));
            }

            let dst = REGISTER_MAP[insn.dst as usize];
            let src = REGISTER_MAP[insn.src as usize];
            let target_pc = (self.pc as isize + insn.off as isize + 1) as usize;

            match insn.opc {
                ebpf::LD_DW_IMM if !self.executable.get_sbpf_version().disable_lddw() => {
                    self.emit_validate_and_profile_instruction_count(Some(self.pc + 2));
                    self.pc += 1;
                    self.result.pc_section[self.pc] = unsafe { self.anchors[ANCHOR_CALL_UNSUPPORTED_INSTRUCTION].offset_from(self.result.text_section.as_ptr()) as u32 };
                    ebpf::augment_lddw_unchecked(self.program, &mut insn);
                    if self.should_sanitize_constant(insn.imm) {
                        self.emit_sanitized_load_immediate(dst, insn.imm);
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(dst, insn.imm));
                    }
                },

                // BPF_LDX class
                ebpf::LD_B_REG  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 1, None);
                },
                ebpf::LD_H_REG  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 2, None);
                },
                ebpf::LD_W_REG  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 4, None);
                },
                ebpf::LD_DW_REG if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 8, None);
                },

                // BPF_ST class
                ebpf::ST_B_IMM  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 1, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::ST_H_IMM  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 2, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::ST_W_IMM  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 4, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::ST_DW_IMM if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 8, Some(Value::Constant64(insn.imm, true)));
                },

                // BPF_STX class
                ebpf::ST_B_REG  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 1, Some(Value::Register(src)));
                },
                ebpf::ST_H_REG  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 2, Some(Value::Register(src)));
                },
                ebpf::ST_W_REG  if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 4, Some(Value::Register(src)));
                },
                ebpf::ST_DW_REG if !self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 8, Some(Value::Register(src)));
                },

                // BPF_ALU32_LOAD class
                ebpf::ADD32_IMM  => {
                    self.emit_sanitized_alu(OperandSize::S32, 0x01, 0, dst, insn.imm);
                    if !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
                    }
                },
                ebpf::ADD32_REG  => {
                    self.emit_ins(X86Instruction::alu(OperandSize::S32, 0x01, src, dst, None));
                    if !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
                    }
                },
                ebpf::SUB32_IMM  => {
                    if self.executable.get_sbpf_version().swap_sub_reg_imm_operands() {
                        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S32, 0xf7, 3, dst, 0, None));
                        if insn.imm != 0 {
                            self.emit_sanitized_alu(OperandSize::S32, 0x01, 0, dst, insn.imm);
                        }
                    } else {
                        self.emit_sanitized_alu(OperandSize::S32, 0x29, 5, dst, insn.imm);
                    }
                    if !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
                    }
                },
                ebpf::SUB32_REG  => {
                    self.emit_ins(X86Instruction::alu(OperandSize::S32, 0x29, src, dst, None));
                    if !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
                    }
                },
                ebpf::MUL32_IMM if !self.executable.get_sbpf_version().enable_pqr() => {
                    if self.should_sanitize_constant(insn.imm) {
                        self.emit_sanitized_load_immediate(REGISTER_SCRATCH, insn.imm);
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, insn.imm));
                    }
                    self.emit_ins(X86Instruction::alu_escaped(OperandSize::S32, 1, 0xaf, dst, REGISTER_SCRATCH, None));
                    if !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
                    }
                },
                ebpf::DIV32_IMM | ebpf::MOD32_IMM if !self.executable.get_sbpf_version().enable_pqr() =>
                    self.emit_product_quotient_remainder(
                        OperandSize::S32,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MOD,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) != ebpf::BPF_MUL,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MUL,
                        dst, dst, Some(insn.imm),
                    ),
                ebpf::LD_1B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 1, None);
                },
                ebpf::MUL32_REG if !self.executable.get_sbpf_version().enable_pqr() => {
                    self.emit_ins(X86Instruction::alu_escaped(OperandSize::S32, 1, 0xaf, dst, src, None));
                    if !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
                    }
                },
                ebpf::DIV32_REG | ebpf::MOD32_REG if !self.executable.get_sbpf_version().enable_pqr() =>
                    self.emit_product_quotient_remainder(
                        OperandSize::S32,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MOD,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) != ebpf::BPF_MUL,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MUL,
                        src, dst, None,
                    ),
                ebpf::LD_2B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 2, None);
                },
                ebpf::OR32_IMM   => self.emit_sanitized_alu(OperandSize::S32, 0x09, 1, dst, insn.imm),
                ebpf::OR32_REG   => self.emit_ins(X86Instruction::alu(OperandSize::S32, 0x09, src, dst, None)),
                ebpf::AND32_IMM  => self.emit_sanitized_alu(OperandSize::S32, 0x21, 4, dst, insn.imm),
                ebpf::AND32_REG  => self.emit_ins(X86Instruction::alu(OperandSize::S32, 0x21, src, dst, None)),
                ebpf::LSH32_IMM  => self.emit_shift(OperandSize::S32, 4, REGISTER_SCRATCH, dst, Some(insn.imm)),
                ebpf::LSH32_REG  => self.emit_shift(OperandSize::S32, 4, src, dst, None),
                ebpf::RSH32_IMM  => self.emit_shift(OperandSize::S32, 5, REGISTER_SCRATCH, dst, Some(insn.imm)),
                ebpf::RSH32_REG  => self.emit_shift(OperandSize::S32, 5, src, dst, None),
                ebpf::NEG32      if !self.executable.get_sbpf_version().disable_neg() => self.emit_ins(X86Instruction::alu_immediate(OperandSize::S32, 0xf7, 3, dst, 0, None)),
                ebpf::LD_4B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 4, None);
                },
                ebpf::LD_8B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(Some(dst), Value::RegisterPlusConstant64(src, insn.off as i64, true), 8, None);
                },
                ebpf::XOR32_IMM  => self.emit_sanitized_alu(OperandSize::S32, 0x31, 6, dst, insn.imm),
                ebpf::XOR32_REG  => self.emit_ins(X86Instruction::alu(OperandSize::S32, 0x31, src, dst, None)),
                ebpf::MOV32_IMM  => {
                    if self.should_sanitize_constant(insn.imm) {
                        self.emit_sanitized_load_immediate(dst, insn.imm as u32 as u64 as i64);
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(dst, insn.imm as u32 as u64 as i64));
                    }
                }
                ebpf::MOV32_REG  => {
                    if self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                        self.emit_ins(X86Instruction::mov_with_sign_extension(OperandSize::S64, src, dst));
                    } else {
                        self.emit_ins(X86Instruction::mov(OperandSize::S32, src, dst));
                    }
                }
                ebpf::ARSH32_IMM => self.emit_shift(OperandSize::S32, 7, REGISTER_SCRATCH, dst, Some(insn.imm)),
                ebpf::ARSH32_REG => self.emit_shift(OperandSize::S32, 7, src, dst, None),
                ebpf::LE if !self.executable.get_sbpf_version().disable_le() => {
                    match insn.imm {
                        16 => {
                            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S32, 0x81, 4, dst, 0xffff, None)); // Mask to 16 bit
                        }
                        32 => {
                            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S32, 0x81, 4, dst, -1, None)); // Mask to 32 bit
                        }
                        64 => {}
                        _ => {
                            return Err(EbpfError::InvalidInstruction);
                        }
                    }
                },
                ebpf::BE         => {
                    match insn.imm {
                        16 => {
                            self.emit_ins(X86Instruction::bswap(OperandSize::S16, dst));
                            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S32, 0x81, 4, dst, 0xffff, None)); // Mask to 16 bit
                        }
                        32 => self.emit_ins(X86Instruction::bswap(OperandSize::S32, dst)),
                        64 => self.emit_ins(X86Instruction::bswap(OperandSize::S64, dst)),
                        _ => {
                            return Err(EbpfError::InvalidInstruction);
                        }
                    }
                },

                // BPF_ALU64_STORE class
                ebpf::ADD64_IMM  => self.emit_sanitized_alu(OperandSize::S64, 0x01, 0, dst, insn.imm),
                ebpf::ADD64_REG  => self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, src, dst, None)),
                ebpf::SUB64_IMM  => {
                    if self.executable.get_sbpf_version().swap_sub_reg_imm_operands() {
                        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xf7, 3, dst, 0, None));
                        if insn.imm != 0 {
                            self.emit_sanitized_alu(OperandSize::S64, 0x01, 0, dst, insn.imm);
                        }
                    } else {
                        self.emit_sanitized_alu(OperandSize::S64, 0x29, 5, dst, insn.imm);
                    }
                }
                ebpf::SUB64_REG  => self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x29, src, dst, None)),
                ebpf::MUL64_IMM if !self.executable.get_sbpf_version().enable_pqr() => {
                    if self.should_sanitize_constant(insn.imm) {
                        self.emit_sanitized_load_immediate(REGISTER_SCRATCH, insn.imm);
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, insn.imm));
                    }
                    self.emit_ins(X86Instruction::alu_escaped(OperandSize::S64, 1, 0xaf, dst, REGISTER_SCRATCH, None));
                },
                ebpf::DIV64_IMM | ebpf::MOD64_IMM if !self.executable.get_sbpf_version().enable_pqr() =>
                    self.emit_product_quotient_remainder(
                        OperandSize::S64,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MOD,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) != ebpf::BPF_MUL,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MUL,
                        dst, dst, Some(insn.imm),
                    ),
                ebpf::ST_1B_IMM  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 1, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::ST_2B_IMM  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 2, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::MUL64_REG if !self.executable.get_sbpf_version().enable_pqr() => {
                    self.emit_ins(X86Instruction::alu_escaped(OperandSize::S64, 1, 0xaf, dst, src, None));
                },
                ebpf::DIV64_REG | ebpf::MOD64_REG if !self.executable.get_sbpf_version().enable_pqr() =>
                    self.emit_product_quotient_remainder(
                        OperandSize::S64,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MOD,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) != ebpf::BPF_MUL,
                        (insn.opc & ebpf::BPF_ALU_OP_MASK) == ebpf::BPF_MUL,
                        src, dst, None,
                    ),
                ebpf::ST_1B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 1, Some(Value::Register(src)));
                },
                ebpf::ST_2B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 2, Some(Value::Register(src)));
                },
                ebpf::OR64_IMM   => self.emit_sanitized_alu(OperandSize::S64, 0x09, 1, dst, insn.imm),
                ebpf::OR64_REG   => self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x09, src, dst, None)),
                ebpf::AND64_IMM  => self.emit_sanitized_alu(OperandSize::S64, 0x21, 4, dst, insn.imm),
                ebpf::AND64_REG  => self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x21, src, dst, None)),
                ebpf::LSH64_IMM  => self.emit_shift(OperandSize::S64, 4, REGISTER_SCRATCH, dst, Some(insn.imm)),
                ebpf::LSH64_REG  => self.emit_shift(OperandSize::S64, 4, src, dst, None),
                ebpf::RSH64_IMM  => self.emit_shift(OperandSize::S64, 5, REGISTER_SCRATCH, dst, Some(insn.imm)),
                ebpf::RSH64_REG  => self.emit_shift(OperandSize::S64, 5, src, dst, None),
                ebpf::ST_4B_IMM  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 4, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::NEG64      if !self.executable.get_sbpf_version().disable_neg() => self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xf7, 3, dst, 0, None)),
                ebpf::ST_4B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 4, Some(Value::Register(src)));
                },
                ebpf::ST_8B_IMM  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 8, Some(Value::Constant64(insn.imm, true)));
                },
                ebpf::ST_8B_REG  if self.executable.get_sbpf_version().move_memory_instruction_classes() => {
                    self.emit_address_translation(None, Value::RegisterPlusConstant64(dst, insn.off as i64, true), 8, Some(Value::Register(src)));
                },
                ebpf::XOR64_IMM  => self.emit_sanitized_alu(OperandSize::S64, 0x31, 6, dst, insn.imm),
                ebpf::XOR64_REG  => self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x31, src, dst, None)),
                ebpf::MOV64_IMM  => {
                    if self.should_sanitize_constant(insn.imm) {
                        self.emit_sanitized_load_immediate(dst, insn.imm);
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(dst, insn.imm));
                    }
                }
                ebpf::MOV64_REG  => self.emit_ins(X86Instruction::mov(OperandSize::S64, src, dst)),
                ebpf::ARSH64_IMM => self.emit_shift(OperandSize::S64, 7, REGISTER_SCRATCH, dst, Some(insn.imm)),
                ebpf::ARSH64_REG => self.emit_shift(OperandSize::S64, 7, src, dst, None),
                ebpf::HOR64_IMM if self.executable.get_sbpf_version().disable_lddw() => {
                    self.emit_sanitized_alu(OperandSize::S64, 0x09, 1, dst, (insn.imm as u64).wrapping_shl(32) as i64);
                }

                // BPF_PQR class
                ebpf::LMUL32_IMM | ebpf::LMUL64_IMM | ebpf::UHMUL64_IMM | ebpf::SHMUL64_IMM |
                ebpf::UDIV32_IMM | ebpf::UDIV64_IMM | ebpf::UREM32_IMM | ebpf::UREM64_IMM |
                ebpf::SDIV32_IMM | ebpf::SDIV64_IMM | ebpf::SREM32_IMM | ebpf::SREM64_IMM
                if self.executable.get_sbpf_version().enable_pqr() => {
                    let signed = insn.opc & (1 << 7) != 0;
                    let mut imm = insn.imm;
                    if !signed {
                        imm &= u32::MAX as i64;
                    }
                    self.emit_product_quotient_remainder(
                        if insn.opc & (1 << 4) != 0 { OperandSize::S64 } else { OperandSize::S32 },
                        insn.opc & (1 << 5) != 0,
                        insn.opc & (1 << 6) != 0,
                        signed,
                        dst, dst, Some(imm),
                    )
                }
                ebpf::LMUL32_REG | ebpf::LMUL64_REG | ebpf::UHMUL64_REG | ebpf::SHMUL64_REG |
                ebpf::UDIV32_REG | ebpf::UDIV64_REG | ebpf::UREM32_REG | ebpf::UREM64_REG |
                ebpf::SDIV32_REG | ebpf::SDIV64_REG | ebpf::SREM32_REG | ebpf::SREM64_REG
                if self.executable.get_sbpf_version().enable_pqr() =>
                    self.emit_product_quotient_remainder(
                        if insn.opc & (1 << 4) != 0 { OperandSize::S64 } else { OperandSize::S32 },
                        insn.opc & (1 << 5) != 0,
                        insn.opc & (1 << 6) != 0,
                        insn.opc & (1 << 7) != 0,
                        src, dst, None,
                    ),

                // BPF_JMP class
                ebpf::JA         => {
                    self.emit_validate_and_profile_instruction_count(Some(target_pc));
                    self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, target_pc as i64));
                    let jump_offset = self.relative_to_target_pc(target_pc, 5);
                    self.emit_ins(X86Instruction::jump_immediate(jump_offset));
                },
                ebpf::JEQ_IMM    => self.emit_conditional_branch_imm(0x84, false, insn.imm, dst, target_pc),
                ebpf::JEQ_REG    => self.emit_conditional_branch_reg(0x84, false, src, dst, target_pc),
                ebpf::JGT_IMM    => self.emit_conditional_branch_imm(0x87, false, insn.imm, dst, target_pc),
                ebpf::JGT_REG    => self.emit_conditional_branch_reg(0x87, false, src, dst, target_pc),
                ebpf::JGE_IMM    => self.emit_conditional_branch_imm(0x83, false, insn.imm, dst, target_pc),
                ebpf::JGE_REG    => self.emit_conditional_branch_reg(0x83, false, src, dst, target_pc),
                ebpf::JLT_IMM    => self.emit_conditional_branch_imm(0x82, false, insn.imm, dst, target_pc),
                ebpf::JLT_REG    => self.emit_conditional_branch_reg(0x82, false, src, dst, target_pc),
                ebpf::JLE_IMM    => self.emit_conditional_branch_imm(0x86, false, insn.imm, dst, target_pc),
                ebpf::JLE_REG    => self.emit_conditional_branch_reg(0x86, false, src, dst, target_pc),
                ebpf::JSET_IMM   => self.emit_conditional_branch_imm(0x85, true, insn.imm, dst, target_pc),
                ebpf::JSET_REG   => self.emit_conditional_branch_reg(0x85, true, src, dst, target_pc),
                ebpf::JNE_IMM    => self.emit_conditional_branch_imm(0x85, false, insn.imm, dst, target_pc),
                ebpf::JNE_REG    => self.emit_conditional_branch_reg(0x85, false, src, dst, target_pc),
                ebpf::JSGT_IMM   => self.emit_conditional_branch_imm(0x8f, false, insn.imm, dst, target_pc),
                ebpf::JSGT_REG   => self.emit_conditional_branch_reg(0x8f, false, src, dst, target_pc),
                ebpf::JSGE_IMM   => self.emit_conditional_branch_imm(0x8d, false, insn.imm, dst, target_pc),
                ebpf::JSGE_REG   => self.emit_conditional_branch_reg(0x8d, false, src, dst, target_pc),
                ebpf::JSLT_IMM   => self.emit_conditional_branch_imm(0x8c, false, insn.imm, dst, target_pc),
                ebpf::JSLT_REG   => self.emit_conditional_branch_reg(0x8c, false, src, dst, target_pc),
                ebpf::JSLE_IMM   => self.emit_conditional_branch_imm(0x8e, false, insn.imm, dst, target_pc),
                ebpf::JSLE_REG   => self.emit_conditional_branch_reg(0x8e, false, src, dst, target_pc),
                ebpf::CALL_IMM => {
                    // For JIT, external functions MUST be registered at compile time.
                    let key = self
                        .executable
                        .get_sbpf_version()
                        .calculate_call_imm_target_pc(self.pc, insn.imm);
                    if self.executable.get_sbpf_version().static_syscalls() {
                        // BPF to BPF call
                        self.emit_internal_call(Value::Constant64(key as i64, true));
                    } else if let Some((_, function)) =
                            self.executable.get_loader().get_function_registry().lookup_by_key(insn.imm as u32) {
                        // SBPFv0 syscall
                        self.emit_syscall_dispatch(function);
                    } else if let Some((_function_name, target_pc)) =
                            self.executable
                                .get_function_registry()
                                .lookup_by_key(key) {
                        // BPF to BPF call
                        self.emit_internal_call(Value::Constant64(target_pc as i64, true));
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.pc as i64));
                        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_CALL_UNSUPPORTED_INSTRUCTION, 5)));
                    }
                },
                ebpf::SYSCALL if self.executable.get_sbpf_version().static_syscalls() => {
                    if let Some((_, function)) = self.executable.get_loader().get_function_registry().lookup_by_key(insn.imm as u32) {
                        self.emit_syscall_dispatch(function);
                    } else {
                        debug_assert!(false, "Invalid syscall should have been detected in the verifier.")
                    }
                },
                ebpf::CALL_REG  => {
                    let target_pc = if self.executable.get_sbpf_version().callx_uses_src_reg() {
                        src
                    } else {
                        REGISTER_MAP[insn.imm as usize]
                    };
                    self.emit_internal_call(Value::Register(target_pc));
                },
                ebpf::RETURN
                | ebpf::EXIT      => {
                    if (insn.opc == ebpf::EXIT && self.executable.get_sbpf_version().static_syscalls())
                        || (insn.opc == ebpf::RETURN && !self.executable.get_sbpf_version().static_syscalls()) {
                        return Err(EbpfError::UnsupportedInstruction);
                    }
                    self.emit_validate_and_profile_instruction_count(Some(0));

                    let call_depth_access = X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::CallDepth));
                    // If env.call_depth == 0, we've reached the exit instruction of the entry point
                    self.emit_ins(X86Instruction::cmp_immediate(OperandSize::S32, REGISTER_PTR_TO_VM, 0, Some(call_depth_access)));
                    // we're done
                    self.emit_ins(X86Instruction::conditional_jump_immediate(0x84, self.relative_to_anchor(ANCHOR_EXIT, 6)));

                    // else decrement and update env.call_depth
                    self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 5, REGISTER_PTR_TO_VM, 1, Some(call_depth_access))); // env.call_depth -= 1;

                    // and return
                    self.emit_ins(X86Instruction::return_near());
                },

                _               => return Err(EbpfError::UnsupportedInstruction),
            }

            self.pc += 1;
        }

        // Bumper in case there was no final exit
        if self.offset_in_text_section + MAX_MACHINE_CODE_LENGTH_PER_INSTRUCTION * 2 >= self.result.text_section.len() {
            return Err(EbpfError::ExhaustedTextSegment(self.pc));
        }
        self.emit_validate_and_profile_instruction_count(Some(self.pc + 1));
        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.pc as i64)); // Save pc
        self.emit_set_exception_kind(EbpfError::ExecutionOverrun);
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 5)));

        self.resolve_jumps();
        self.result.seal(self.offset_in_text_section)?;
        Ok(self.result)
    }

    fn should_sanitize_constant(&self, value: i64) -> bool {
        if !self.config.sanitize_user_provided_values {
            return false;
        }

        match value as u64 {
            0xFFFF
            | 0xFFFFFF
            | 0xFFFFFFFF
            | 0xFFFFFFFFFF
            | 0xFFFFFFFFFFFF
            | 0xFFFFFFFFFFFFFF
            | 0xFFFFFFFFFFFFFFFF => false,
            v if v <= 0xFF => false,
            v if !v <= 0xFF => false,
            _ => true
        }
    }

    fn slot_in_vm(&self, slot: RuntimeEnvironmentSlot) -> i32 {
        8 * (slot as i32 - self.runtime_environment_key)
    }

    pub(crate) fn emit<T>(&mut self, data: T) {
        unsafe {
            let ptr = self.result.text_section.as_ptr().add(self.offset_in_text_section);
            #[allow(clippy::cast_ptr_alignment)]
            ptr::write_unaligned(ptr as *mut T, data as T);
        }
        self.offset_in_text_section += mem::size_of::<T>();
    }

    pub(crate) fn emit_variable_length(&mut self, size: OperandSize, data: u64) {
        match size {
            OperandSize::S0 => {},
            OperandSize::S8 => self.emit::<u8>(data as u8),
            OperandSize::S16 => self.emit::<u16>(data as u16),
            OperandSize::S32 => self.emit::<u32>(data as u32),
            OperandSize::S64 => self.emit::<u64>(data),
        }
    }

    // This function helps the optimizer to inline the machinecode emission while avoiding stack allocations
    #[inline(always)]
    fn emit_ins(&mut self, instruction: X86Instruction) {
        instruction.emit(self);
        if self.next_noop_insertion == 0 {
            self.next_noop_insertion = self.noop_range.sample(&mut self.diversification_rng);
            // X86Instruction::noop().emit(self)?;
            self.emit::<u8>(0x90);
        } else {
            self.next_noop_insertion -= 1;
        }
    }

    fn emit_sanitized_load_immediate(&mut self, destination: X86Register, value: i64) {
        let lower_key = self.immediate_value_key as i32 as i64;
        if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
            self.emit_ins(X86Instruction::load_immediate(destination, value.wrapping_sub(lower_key)));
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, destination, lower_key, None)); // wrapping_add(lower_key)
        } else if value as u64 & u32::MAX as u64 == 0 {
            self.emit_ins(X86Instruction::load_immediate(destination, value.rotate_right(32).wrapping_sub(lower_key)));
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, destination, lower_key, None)); // wrapping_add(lower_key)
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xc1, 4, destination, 32, None)); // shift_left(32)
        } else if destination != REGISTER_SCRATCH {
            self.emit_ins(X86Instruction::load_immediate(destination, value.wrapping_sub(self.immediate_value_key)));
            self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.immediate_value_key));
            self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, REGISTER_SCRATCH, destination, None)); // wrapping_add(immediate_value_key)
        } else {
            let upper_key = (self.immediate_value_key >> 32) as i32 as i64;
            self.emit_ins(X86Instruction::load_immediate(destination, value.wrapping_sub(lower_key).rotate_right(32).wrapping_sub(upper_key)));
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, destination, upper_key, None)); // wrapping_add(upper_key)
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xc1, 1, destination, 32, None)); // rotate_right(32)
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, destination, lower_key, None)); // wrapping_add(lower_key)
        }
    }

    fn emit_sanitized_alu(&mut self, size: OperandSize, opcode: u8, opcode_extension: u8, destination: X86Register, immediate: i64) {
        if self.should_sanitize_constant(immediate) {
            self.emit_sanitized_load_immediate(REGISTER_SCRATCH, immediate);
            self.emit_ins(X86Instruction::alu(size, opcode, REGISTER_SCRATCH, destination, None));
        } else if immediate >= i32::MIN as i64 && immediate <= i32::MAX as i64 {
            self.emit_ins(X86Instruction::alu_immediate(size, 0x81, opcode_extension, destination, immediate, None));
        } else {
            self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, immediate));
            self.emit_ins(X86Instruction::alu(size, opcode, REGISTER_SCRATCH, destination, None));
        }
    }

    #[allow(dead_code)]
    fn emit_stopwatch(&mut self, begin: bool) {
        self.stopwatch_is_active = true;
        self.emit_ins(X86Instruction::push(RDX, None));
        self.emit_ins(X86Instruction::push(RAX, None));
        self.emit_ins(X86Instruction::fence(FenceType::Load)); // lfence
        self.emit_ins(X86Instruction::cycle_count()); // rdtsc
        self.emit_ins(X86Instruction::fence(FenceType::Load)); // lfence
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xc1, 4, RDX, 32, None)); // RDX <<= 32;
        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x09, RDX, RAX, None)); // RAX |= RDX;
        if begin {
            self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x29, RAX, REGISTER_PTR_TO_VM, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::StopwatchNumerator))))); // *numerator -= RAX;
        } else {
            self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, RAX, REGISTER_PTR_TO_VM, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::StopwatchNumerator))))); // *numerator += RAX;
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, REGISTER_PTR_TO_VM, 1, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::StopwatchDenominator))))); // *denominator += 1;
        }
        self.emit_ins(X86Instruction::pop(RAX));
        self.emit_ins(X86Instruction::pop(RDX));
    }

    fn emit_validate_instruction_count(&mut self, pc: Option<usize>) {
        if !self.config.enable_instruction_meter {
            return;
        }
        // Update `MACHINE_CODE_PER_INSTRUCTION_METER_CHECKPOINT` if you change the code generation here
        if let Some(pc) = pc {
            self.last_instruction_meter_validation_pc = pc;
            self.emit_sanitized_load_immediate(REGISTER_SCRATCH, pc as i64);
        }
        // If instruction_meter >= pc, throw ExceededMaxInstructions
        self.emit_ins(X86Instruction::cmp(OperandSize::S64, REGISTER_SCRATCH, REGISTER_INSTRUCTION_METER, None));
        self.emit_ins(X86Instruction::conditional_jump_immediate(0x86, self.relative_to_anchor(ANCHOR_THROW_EXCEEDED_MAX_INSTRUCTIONS, 6)));
    }

    fn emit_profile_instruction_count(&mut self, target_pc: Option<usize>) {
        if !self.config.enable_instruction_meter {
            return;
        }
        match target_pc {
            Some(target_pc) => {
                self.emit_sanitized_alu(OperandSize::S64, 0x01, 0, REGISTER_INSTRUCTION_METER, target_pc as i64 - self.pc as i64 - 1); // instruction_meter += target_pc - (self.pc + 1);
            },
            None => {
                self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, REGISTER_SCRATCH, REGISTER_INSTRUCTION_METER, None)); // instruction_meter += target_pc;
                self.emit_sanitized_alu(OperandSize::S64, 0x81, 5, REGISTER_INSTRUCTION_METER, self.pc as i64 + 1); // instruction_meter -= self.pc + 1;
            },
        }
    }

    fn emit_undo_profile_instruction_count(&mut self, target_pc: usize) {
        if self.config.enable_instruction_meter {
            self.emit_sanitized_alu(OperandSize::S64, 0x01, 0, REGISTER_INSTRUCTION_METER, self.pc as i64 + 1 - target_pc as i64); // instruction_meter += (self.pc + 1) - target_pc;
        }
    }

    fn emit_validate_and_profile_instruction_count(&mut self, target_pc: Option<usize>) {
        self.emit_validate_instruction_count(Some(self.pc));
        self.emit_profile_instruction_count(target_pc);
    }

    fn emit_rust_call(&mut self, target: Value, arguments: &[Argument], result_reg: Option<X86Register>) {
        let mut saved_registers = CALLER_SAVED_REGISTERS.to_vec();
        if let Some(reg) = result_reg {
            if let Some(dst) = saved_registers.iter().position(|x| *x == reg) {
                saved_registers.remove(dst);
            }
        }
    
        // Save registers on stack
        for reg in saved_registers.iter() {
            self.emit_ins(X86Instruction::push(*reg, None));
        }

        let stack_arguments = arguments.len().saturating_sub(ARGUMENT_REGISTERS.len()) as i64;
        if stack_arguments % 2 != 0 {
            // If we're going to pass an odd number of stack args we need to pad
            // to preserve alignment
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 5, RSP, 8, None));
        }

        // Pass arguments
        for argument in arguments {
            let is_stack_argument = argument.index >= ARGUMENT_REGISTERS.len();
            let dst = if is_stack_argument {
                RSP // Never used
            } else {
                ARGUMENT_REGISTERS[argument.index]
            };
            match argument.value {
                Value::Register(reg) => {
                    if is_stack_argument {
                        self.emit_ins(X86Instruction::push(reg, None));
                    } else if reg != dst {
                        self.emit_ins(X86Instruction::mov(OperandSize::S64, reg, dst));
                    }
                },
                Value::RegisterIndirect(reg, offset, user_provided) => {
                    debug_assert!(!user_provided);
                    if is_stack_argument {
                        debug_assert!(reg != RSP);
                        self.emit_ins(X86Instruction::push(reg, Some(X86IndirectAccess::Offset(offset))));
                    } else if reg == RSP {
                        self.emit_ins(X86Instruction::load(OperandSize::S64, RSP, dst, X86IndirectAccess::OffsetIndexShift(offset, RSP, 0)));
                    } else {
                        self.emit_ins(X86Instruction::load(OperandSize::S64, reg, dst, X86IndirectAccess::Offset(offset)));
                    }
                },
                Value::RegisterPlusConstant32(reg, offset, user_provided) => {
                    debug_assert!(!user_provided);
                    if is_stack_argument {
                        self.emit_ins(X86Instruction::push(reg, None));
                        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP, offset as i64, Some(X86IndirectAccess::OffsetIndexShift(0, RSP, 0))));
                    } else if reg == RSP {
                        self.emit_ins(X86Instruction::lea(OperandSize::S64, RSP, dst, Some(X86IndirectAccess::OffsetIndexShift(offset, RSP, 0))));
                    } else {
                        self.emit_ins(X86Instruction::lea(OperandSize::S64, reg, dst, Some(X86IndirectAccess::Offset(offset))));
                    }
                },
                Value::RegisterPlusConstant64(reg, offset, user_provided) => {
                    debug_assert!(!user_provided);
                    if is_stack_argument {
                        self.emit_ins(X86Instruction::push(reg, None));
                        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP, offset, Some(X86IndirectAccess::OffsetIndexShift(0, RSP, 0))));
                    } else {
                        self.emit_ins(X86Instruction::load_immediate(dst, offset));
                        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, reg, dst, None));
                    }
                },
                Value::Constant64(value, user_provided) => {
                    debug_assert!(!user_provided && !is_stack_argument);
                    self.emit_ins(X86Instruction::load_immediate(dst, value));
                },
            }
        }
    
        match target {
            Value::Register(reg) => {
                self.emit_ins(X86Instruction::call_reg(reg, None));
            },
            Value::Constant64(value, user_provided) => {
                debug_assert!(!user_provided);
                self.emit_ins(X86Instruction::load_immediate(RAX, value));
                self.emit_ins(X86Instruction::call_reg(RAX, None));
            },
            _ => {
                #[cfg(debug_assertions)]
                unreachable!();
            }
        }
    
        // Save returned value in result register
        if let Some(reg) = result_reg {
            self.emit_ins(X86Instruction::mov(OperandSize::S64, RAX, reg));
        }
    
        // Restore registers from stack
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP,
            if stack_arguments % 2 != 0 { stack_arguments + 1 } else { stack_arguments } * 8, None));

        for reg in saved_registers.iter().rev() {
            self.emit_ins(X86Instruction::pop(*reg));
        }
    }

    fn emit_internal_call(&mut self, dst: Value) {
        // Store PC in case the bounds check fails
        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.pc as i64));
        self.last_instruction_meter_validation_pc = self.pc;
        self.emit_ins(X86Instruction::call_immediate(self.relative_to_anchor(ANCHOR_INTERNAL_FUNCTION_CALL_PROLOGUE, 5)));

        match dst {
            Value::Register(reg) => {
                // REGISTER_SCRATCH contains self.pc, and we must store it for proper error handling.
                // We can discard the value if callx succeeds, so we are not incrementing the stack pointer (RSP).
                self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_SCRATCH, RSP, X86IndirectAccess::OffsetIndexShift(-24, RSP, 0)));
                // Move guest_target_address into REGISTER_SCRATCH
                self.emit_ins(X86Instruction::mov(OperandSize::S64, reg, REGISTER_SCRATCH));
                self.emit_ins(X86Instruction::call_immediate(self.relative_to_anchor(ANCHOR_INTERNAL_FUNCTION_CALL_REG, 5)));
            },
            Value::Constant64(target_pc, user_provided) => {
                debug_assert!(user_provided);
                self.emit_profile_instruction_count(Some(target_pc as usize));
                if user_provided && self.should_sanitize_constant(target_pc) {
                    self.emit_sanitized_load_immediate(REGISTER_SCRATCH, target_pc);
                } else {
                    self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, target_pc));
                }
                let jump_offset = self.relative_to_target_pc(target_pc as usize, 5);
                self.emit_ins(X86Instruction::call_immediate(jump_offset));
            },
            _ => {
                #[cfg(debug_assertions)]
                unreachable!();
            }
        }

        self.emit_undo_profile_instruction_count(0);

        // Restore the previous frame pointer
        self.emit_ins(X86Instruction::pop(REGISTER_MAP[FRAME_PTR_REG]));
        for reg in REGISTER_MAP.iter().skip(FIRST_SCRATCH_REG).take(SCRATCH_REGS).rev() {
            self.emit_ins(X86Instruction::pop(*reg));
        }
    }

    fn emit_syscall_dispatch(&mut self, function: BuiltinFunction<C>) {
        self.emit_validate_and_profile_instruction_count(Some(0));
        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, function as usize as i64));
        self.emit_ins(X86Instruction::call_immediate(self.relative_to_anchor(ANCHOR_EXTERNAL_FUNCTION_CALL, 5)));
        self.emit_undo_profile_instruction_count(0);
    }

    fn emit_address_translation(&mut self, dst: Option<X86Register>, vm_addr: Value, len: u64, value: Option<Value>) {
        debug_assert_ne!(dst.is_some(), value.is_some());

        let stack_slot_of_value_to_store = X86IndirectAccess::OffsetIndexShift(-96, RSP, 0);
        match value {
            Some(Value::Register(reg)) => {
                self.emit_ins(X86Instruction::store(OperandSize::S64, reg, RSP, stack_slot_of_value_to_store));
            }
            Some(Value::Constant64(constant, user_provided)) => {
                debug_assert!(user_provided);
                // First half of emit_sanitized_load_immediate(stack_slot_of_value_to_store, constant)
                let lower_key = self.immediate_value_key as i32 as i64;
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, constant.wrapping_sub(lower_key)));
                self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_SCRATCH, RSP, stack_slot_of_value_to_store));
            }
            _ => {}
        }

        match vm_addr {
            Value::RegisterPlusConstant64(reg, constant, user_provided) => {
                if user_provided && self.should_sanitize_constant(constant) {
                    self.emit_sanitized_load_immediate(REGISTER_SCRATCH, constant);
                } else {
                    self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, constant));
                }
                self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, reg, REGISTER_SCRATCH, None));
            },
            _ => {
                #[cfg(debug_assertions)]
                unreachable!();
            },
        }

        if self.config.enable_address_translation {
            let anchor_base = match value {
                Some(Value::Register(_reg)) => 4,
                Some(Value::Constant64(_constant, _user_provided)) => 8,
                _ => 0,
            };
            let anchor = ANCHOR_TRANSLATE_MEMORY_ADDRESS + anchor_base + len.trailing_zeros() as usize;
            self.emit_ins(X86Instruction::push_immediate(OperandSize::S64, self.pc as i32));
            self.emit_ins(X86Instruction::call_immediate(self.relative_to_anchor(anchor, 5)));
            if let Some(dst) = dst {
                self.emit_ins(X86Instruction::mov(OperandSize::S64, REGISTER_SCRATCH, dst));
            }
        } else if let Some(dst) = dst {
            match len {
                1 => self.emit_ins(X86Instruction::load(OperandSize::S8, REGISTER_SCRATCH, dst, X86IndirectAccess::Offset(0))),
                2 => self.emit_ins(X86Instruction::load(OperandSize::S16, REGISTER_SCRATCH, dst, X86IndirectAccess::Offset(0))),
                4 => self.emit_ins(X86Instruction::load(OperandSize::S32, REGISTER_SCRATCH, dst, X86IndirectAccess::Offset(0))),
                8 => self.emit_ins(X86Instruction::load(OperandSize::S64, REGISTER_SCRATCH, dst, X86IndirectAccess::Offset(0))),
                _ => unreachable!(),
            }
        } else {
            self.emit_ins(X86Instruction::xchg(OperandSize::S64, RSP, REGISTER_MAP[0], Some(stack_slot_of_value_to_store))); // Save REGISTER_MAP[0] and retrieve value to store
            match len {
                1 => self.emit_ins(X86Instruction::store(OperandSize::S8, REGISTER_MAP[0], REGISTER_SCRATCH, X86IndirectAccess::Offset(0))),
                2 => self.emit_ins(X86Instruction::store(OperandSize::S16, REGISTER_MAP[0], REGISTER_SCRATCH, X86IndirectAccess::Offset(0))),
                4 => self.emit_ins(X86Instruction::store(OperandSize::S32, REGISTER_MAP[0], REGISTER_SCRATCH, X86IndirectAccess::Offset(0))),
                8 => self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_MAP[0], REGISTER_SCRATCH, X86IndirectAccess::Offset(0))),
                _ => unreachable!(),
            }
            self.emit_ins(X86Instruction::xchg(OperandSize::S64, RSP, REGISTER_MAP[0], Some(stack_slot_of_value_to_store))); // Restore REGISTER_MAP[0]
        }
    }

    fn emit_conditional_branch_reg(&mut self, op: u8, bitwise: bool, first_operand: X86Register, second_operand: X86Register, target_pc: usize) {
        self.emit_validate_and_profile_instruction_count(Some(target_pc));
        if bitwise { // Logical
            self.emit_ins(X86Instruction::test(OperandSize::S64, first_operand, second_operand, None));
        } else { // Arithmetic
            self.emit_ins(X86Instruction::cmp(OperandSize::S64, first_operand, second_operand, None));
        }
        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, target_pc as i64));
        let jump_offset = self.relative_to_target_pc(target_pc, 6);
        self.emit_ins(X86Instruction::conditional_jump_immediate(op, jump_offset));
        self.emit_undo_profile_instruction_count(target_pc);
    }

    fn emit_conditional_branch_imm(&mut self, op: u8, bitwise: bool, immediate: i64, second_operand: X86Register, target_pc: usize) {
        self.emit_validate_and_profile_instruction_count(Some(target_pc));
        if self.should_sanitize_constant(immediate) {
            self.emit_sanitized_load_immediate(REGISTER_SCRATCH, immediate);
            if bitwise { // Logical
                self.emit_ins(X86Instruction::test(OperandSize::S64, REGISTER_SCRATCH, second_operand, None));
            } else { // Arithmetic
                self.emit_ins(X86Instruction::cmp(OperandSize::S64, REGISTER_SCRATCH, second_operand, None));
            }
        } else if bitwise { // Logical
            self.emit_ins(X86Instruction::test_immediate(OperandSize::S64, second_operand, immediate, None));
        } else { // Arithmetic
            self.emit_ins(X86Instruction::cmp_immediate(OperandSize::S64, second_operand, immediate, None));
        }
        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, target_pc as i64));
        let jump_offset = self.relative_to_target_pc(target_pc, 6);
        self.emit_ins(X86Instruction::conditional_jump_immediate(op, jump_offset));
        self.emit_undo_profile_instruction_count(target_pc);
    }

    fn emit_shift(&mut self, size: OperandSize, opcode_extension: u8, source: X86Register, destination: X86Register, immediate: Option<i64>) {
        if let Some(immediate) = immediate {
            self.emit_ins(X86Instruction::alu_immediate(size, 0xc1, opcode_extension, destination, immediate, None));
            return;
        }
        if let OperandSize::S32 = size {
            self.emit_ins(X86Instruction::mov(OperandSize::S32, destination, destination)); // Truncate to 32 bit
        }
        if source == RCX {
            self.emit_ins(X86Instruction::alu_immediate(size, 0xd3, opcode_extension, destination, 0, None));
        } else if destination == RCX {
            self.emit_ins(X86Instruction::push(source, None));
            self.emit_ins(X86Instruction::xchg(OperandSize::S64, source, RCX, None));
            self.emit_ins(X86Instruction::alu_immediate(size, 0xd3, opcode_extension, source, 0, None));
            self.emit_ins(X86Instruction::mov(OperandSize::S64, source, RCX));
            self.emit_ins(X86Instruction::pop(source));
        } else {
            self.emit_ins(X86Instruction::push(RCX, None));
            self.emit_ins(X86Instruction::mov(OperandSize::S64, source, RCX));
            self.emit_ins(X86Instruction::alu_immediate(size, 0xd3, opcode_extension, destination, 0, None));
            self.emit_ins(X86Instruction::pop(RCX));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_product_quotient_remainder(
        &mut self,
        size: OperandSize,
        alt_dst: bool,
        division: bool,
        signed: bool,
        src: X86Register,
        dst: X86Register,
        imm: Option<i64>,
    ) {
        //         LMUL UHMUL SHMUL UDIV SDIV UREM SREM
        // ALU     F7/4 F7/4  F7/5  F7/6 F7/7 F7/6 F7/7
        // src-in  REGISTER_SCRATCH  REGISTER_SCRATCH   REGISTER_SCRATCH   REGISTER_SCRATCH  REGISTER_SCRATCH  REGISTER_SCRATCH  REGISTER_SCRATCH
        // dst-in  RAX  RAX   RAX   RAX  RAX  RAX  RAX
        // dst-out RAX  RDX   RDX   RAX  RAX  RDX  RDX

        if division {
            // Prevent division by zero
            if imm.is_none() {
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.pc as i64)); // Save pc
                self.emit_ins(X86Instruction::test(size, src, src, None)); // src == 0
                self.emit_ins(X86Instruction::conditional_jump_immediate(0x84, self.relative_to_anchor(ANCHOR_DIV_BY_ZERO, 6)));
            }

            // Signed division overflows with MIN / -1.
            // If we have an immediate and it's not -1, we can skip the following check.
            if signed && imm.unwrap_or(-1) == -1 {
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, if let OperandSize::S64 = size { i64::MIN } else { i32::MIN as i64 }));
                self.emit_ins(X86Instruction::cmp(size, dst, REGISTER_SCRATCH, None)); // dst == MIN

                if imm.is_none() {
                    // The exception case is: dst == MIN && src == -1
                    // Via De Morgan's law becomes: !(dst != MIN || src != -1)
                    // Also, we know that src != 0 in here, so we can use it to set REGISTER_SCRATCH to something not zero
                    self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, 0)); // No XOR here because we need to keep the status flags
                    self.emit_ins(X86Instruction::cmov(size, 0x45, src, REGISTER_SCRATCH)); // if dst != MIN { REGISTER_SCRATCH = src; }
                    self.emit_ins(X86Instruction::cmp_immediate(size, src, -1, None)); // src == -1
                    self.emit_ins(X86Instruction::cmov(size, 0x45, src, REGISTER_SCRATCH)); // if src != -1 { REGISTER_SCRATCH = src; }
                    self.emit_ins(X86Instruction::test(size, REGISTER_SCRATCH, REGISTER_SCRATCH, None)); // REGISTER_SCRATCH == 0
                }

                // MIN / -1, raise EbpfError::DivideOverflow
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.pc as i64));
                self.emit_ins(X86Instruction::conditional_jump_immediate(0x84, self.relative_to_anchor(ANCHOR_DIV_OVERFLOW, 6)));
            }
        }

        if let Some(imm) = imm {
            if self.should_sanitize_constant(imm) {
                self.emit_sanitized_load_immediate(REGISTER_SCRATCH, imm);
            } else {
                self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, imm));
            }
        } else {
            self.emit_ins(X86Instruction::mov(OperandSize::S64, src, REGISTER_SCRATCH));
        }
        if dst != RAX {
            self.emit_ins(X86Instruction::push(RAX, None));
            self.emit_ins(X86Instruction::mov(OperandSize::S64, dst, RAX));
        }
        if dst != RDX {
            self.emit_ins(X86Instruction::push(RDX, None));
        }
        if division {
            if signed {
                self.emit_ins(X86Instruction::sign_extend_rax_rdx(size));
            } else {
                self.emit_ins(X86Instruction::alu(size, 0x31, RDX, RDX, None)); // RDX = 0
            }
        }

        self.emit_ins(X86Instruction::alu_immediate(size, 0xf7, 0x4 | ((division as u8) << 1) | signed as u8, REGISTER_SCRATCH, 0, None));

        if dst != RDX {
            if alt_dst {
                self.emit_ins(X86Instruction::mov(OperandSize::S64, RDX, dst));
            }
            self.emit_ins(X86Instruction::pop(RDX));
        }
        if dst != RAX {
            if !alt_dst {
                self.emit_ins(X86Instruction::mov(OperandSize::S64, RAX, dst));
            }
            self.emit_ins(X86Instruction::pop(RAX));
        }
        if let OperandSize::S32 = size {
            if signed && !self.executable.get_sbpf_version().explicit_sign_extension_of_results() {
                self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x63, dst, dst, None)); // sign extend i32 to i64
            }
        }
    }

    fn emit_set_exception_kind(&mut self, err: EbpfError) {
        let err_kind = unsafe { *std::ptr::addr_of!(err).cast::<u64>() };
        let err_discriminant = ProgramResult::Err(err).discriminant();
        self.emit_ins(X86Instruction::lea(OperandSize::S64, REGISTER_PTR_TO_VM, REGISTER_MAP[0], Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult)))));
        self.emit_ins(X86Instruction::store_immediate(OperandSize::S64, REGISTER_MAP[0], X86IndirectAccess::Offset(0), err_discriminant as i64)); // result.discriminant = err_discriminant;
        self.emit_ins(X86Instruction::store_immediate(OperandSize::S64, REGISTER_MAP[0], X86IndirectAccess::Offset(std::mem::size_of::<u64>() as i32), err_kind as i64)); // err.kind = err_kind;
    }

    fn emit_result_is_err(&mut self, destination: X86Register) {
        let ok = ProgramResult::Ok(0);
        let ok_discriminant = ok.discriminant();
        self.emit_ins(X86Instruction::lea(OperandSize::S64, REGISTER_PTR_TO_VM, destination, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult)))));
        self.emit_ins(X86Instruction::cmp_immediate(OperandSize::S64, destination, ok_discriminant as i64, Some(X86IndirectAccess::Offset(0))));
    }

    fn emit_subroutines(&mut self) {
        // Routine for instruction tracing
        if self.config.enable_register_tracing {
            self.set_anchor(ANCHOR_TRACE);
            // Save registers on stack
            self.emit_ins(X86Instruction::push(REGISTER_SCRATCH, None));
            for reg in REGISTER_MAP.iter().rev() {
                self.emit_ins(X86Instruction::push(*reg, None));
            }
            self.emit_ins(X86Instruction::mov(OperandSize::S64, RSP, REGISTER_MAP[0]));
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP, - 8 * 3, None)); // RSP -= 8 * 3;
            self.emit_rust_call(Value::Constant64(Vec::<crate::static_analysis::RegisterTraceEntry>::push as *const u8 as i64, false), &[
                Argument { index: 1, value: Value::Register(REGISTER_MAP[0]) }, // registers
                Argument { index: 0, value: Value::RegisterPlusConstant32(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::RegisterTrace), false) },
            ], None);
            // Pop stack and return
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP, 8 * 3, None)); // RSP += 8 * 3;
            self.emit_ins(X86Instruction::pop(REGISTER_MAP[0]));
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP, 8 * (REGISTER_MAP.len() - 1) as i64, None)); // RSP += 8 * (REGISTER_MAP.len() - 1);
            self.emit_ins(X86Instruction::pop(REGISTER_SCRATCH));
            self.emit_ins(X86Instruction::return_near());
        }

        // Epilogue
        self.set_anchor(ANCHOR_EPILOGUE);
        if self.config.enable_instruction_meter {
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 5, REGISTER_INSTRUCTION_METER, 1, None)); // REGISTER_INSTRUCTION_METER -= 1;
            self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x29, REGISTER_SCRATCH, REGISTER_INSTRUCTION_METER, None)); // REGISTER_INSTRUCTION_METER -= pc;
            // *DueInsnCount = *PreviousInstructionMeter - REGISTER_INSTRUCTION_METER;
            self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x2B, REGISTER_INSTRUCTION_METER, REGISTER_PTR_TO_VM, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::PreviousInstructionMeter))))); // REGISTER_INSTRUCTION_METER -= *PreviousInstructionMeter;
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xf7, 3, REGISTER_INSTRUCTION_METER, 0, None)); // REGISTER_INSTRUCTION_METER = -REGISTER_INSTRUCTION_METER;
            self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_INSTRUCTION_METER, REGISTER_PTR_TO_VM, X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::DueInsnCount)))); // *DueInsnCount = REGISTER_INSTRUCTION_METER;
        }
        // Print stop watch value
        fn stopwatch_result(numerator: u64, denominator: u64) {
            println!("Stop watch: {} / {} = {}", numerator, denominator, if denominator == 0 { 0.0 } else { numerator as f64 / denominator as f64 });
        }
        if self.stopwatch_is_active {
            self.emit_rust_call(Value::Constant64(stopwatch_result as *const u8 as i64, false), &[
                Argument { index: 1, value: Value::RegisterIndirect(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::StopwatchDenominator), false) },
                Argument { index: 0, value: Value::RegisterIndirect(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::StopwatchNumerator), false) },
            ], None);
        }
        // Restore stack pointer in case we did not exit gracefully
        self.emit_ins(X86Instruction::load(OperandSize::S64, REGISTER_PTR_TO_VM, RSP, X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::HostStackPointer))));
        self.emit_ins(X86Instruction::return_near());

        // Handler for EbpfError::ExceededMaxInstructions
        self.set_anchor(ANCHOR_THROW_EXCEEDED_MAX_INSTRUCTIONS);
        self.emit_set_exception_kind(EbpfError::ExceededMaxInstructions);
        self.emit_ins(X86Instruction::mov(OperandSize::S64, REGISTER_INSTRUCTION_METER, REGISTER_SCRATCH)); // REGISTER_SCRATCH = REGISTER_INSTRUCTION_METER;
        // Fall through

        // Epilogue for errors
        self.set_anchor(ANCHOR_THROW_EXCEPTION_UNCHECKED);
        self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_SCRATCH, REGISTER_PTR_TO_VM, X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::Registers) + 11 * std::mem::size_of::<u64>() as i32))); // registers[11] = pc;
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_EPILOGUE, 5)));

        // Quit gracefully
        self.set_anchor(ANCHOR_EXIT);
        if self.config.enable_instruction_meter {
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, REGISTER_INSTRUCTION_METER, 1, None)); // REGISTER_INSTRUCTION_METER += 1;
        }
        self.emit_ins(X86Instruction::lea(OperandSize::S64, REGISTER_PTR_TO_VM, REGISTER_SCRATCH, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult)))));
        self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_MAP[0], REGISTER_SCRATCH, X86IndirectAccess::Offset(std::mem::size_of::<u64>() as i32))); // result.return_value = R0;
        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x31, REGISTER_SCRATCH, REGISTER_SCRATCH, None)); // REGISTER_SCRATCH ^= REGISTER_SCRATCH; // REGISTER_SCRATCH = 0;
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_EPILOGUE, 5)));

        // Handler for exceptions which report their pc
        self.set_anchor(ANCHOR_THROW_EXCEPTION);
        // Validate that we did not reach the instruction meter limit before the exception occured
        self.emit_validate_instruction_count(None);
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION_UNCHECKED, 5)));

        // Handler for EbpfError::CallDepthExceeded
        self.set_anchor(ANCHOR_CALL_DEPTH_EXCEEDED);
        self.emit_set_exception_kind(EbpfError::CallDepthExceeded);
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 5)));

        // Handler for EbpfError::CallOutsideTextSegment
        self.set_anchor(ANCHOR_CALL_OUTSIDE_TEXT_SEGMENT);
        self.emit_set_exception_kind(EbpfError::CallOutsideTextSegment);
        self.emit_ins(X86Instruction::load(OperandSize::S64, RSP, REGISTER_SCRATCH, X86IndirectAccess::OffsetIndexShift(-8, RSP, 0)));
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 5)));

        // Handler for EbpfError::DivideByZero
        self.set_anchor(ANCHOR_DIV_BY_ZERO);
        self.emit_set_exception_kind(EbpfError::DivideByZero);
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 5)));

        // Handler for EbpfError::DivideOverflow
        self.set_anchor(ANCHOR_DIV_OVERFLOW);
        self.emit_set_exception_kind(EbpfError::DivideOverflow);
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 5)));

        // See `ANCHOR_INTERNAL_FUNCTION_CALL_REG` for more details.
        self.set_anchor(ANCHOR_CALL_REG_UNSUPPORTED_INSTRUCTION);
        self.emit_ins(X86Instruction::load(OperandSize::S64, RSP, REGISTER_SCRATCH, X86IndirectAccess::OffsetIndexShift(-8, RSP, 0))); // Retrieve the current program counter from the stack
        self.emit_ins(X86Instruction::pop(REGISTER_MAP[0])); // Restore the clobbered REGISTER_MAP[0]
        // Fall through

        // Handler for EbpfError::UnsupportedInstruction
        self.set_anchor(ANCHOR_CALL_UNSUPPORTED_INSTRUCTION);
        if self.config.enable_register_tracing {
            self.emit_ins(X86Instruction::call_immediate(self.relative_to_anchor(ANCHOR_TRACE, 5)));
        }
        self.emit_set_exception_kind(EbpfError::UnsupportedInstruction);
        self.emit_ins(X86Instruction::jump_immediate(self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 5)));

        // Routine for external functions
        self.set_anchor(ANCHOR_EXTERNAL_FUNCTION_CALL);
        self.emit_ins(X86Instruction::push_immediate(OperandSize::S64, -1)); // Used as PC value in error case, acts as stack padding otherwise
        if self.config.enable_instruction_meter {
            self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_INSTRUCTION_METER, REGISTER_PTR_TO_VM, X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::DueInsnCount)))); // *DueInsnCount = REGISTER_INSTRUCTION_METER;
        }
        self.emit_rust_call(Value::Register(REGISTER_SCRATCH), &[
            Argument { index: 5, value: Value::Register(ARGUMENT_REGISTERS[5]) },
            Argument { index: 4, value: Value::Register(ARGUMENT_REGISTERS[4]) },
            Argument { index: 3, value: Value::Register(ARGUMENT_REGISTERS[3]) },
            Argument { index: 2, value: Value::Register(ARGUMENT_REGISTERS[2]) },
            Argument { index: 1, value: Value::Register(ARGUMENT_REGISTERS[1]) },
            Argument { index: 0, value: Value::Register(REGISTER_PTR_TO_VM) },
        ], None);
        if self.config.enable_instruction_meter {
            self.emit_ins(X86Instruction::load(OperandSize::S64, REGISTER_PTR_TO_VM, REGISTER_INSTRUCTION_METER, X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::PreviousInstructionMeter)))); // REGISTER_INSTRUCTION_METER = *PreviousInstructionMeter;
        }

        // Test if result indicates that an error occured
        self.emit_result_is_err(REGISTER_SCRATCH);
        self.emit_ins(X86Instruction::pop(REGISTER_SCRATCH));
        self.emit_ins(X86Instruction::conditional_jump_immediate(0x85, self.relative_to_anchor(ANCHOR_EPILOGUE, 6)));
        // Store Ok value in result register
        self.emit_ins(X86Instruction::lea(OperandSize::S64, REGISTER_PTR_TO_VM, REGISTER_SCRATCH, Some(X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult)))));
        self.emit_ins(X86Instruction::load(OperandSize::S64, REGISTER_SCRATCH, REGISTER_MAP[0], X86IndirectAccess::Offset(8)));
        self.emit_ins(X86Instruction::return_near());

        // Routine for prologue of emit_internal_call()
        self.set_anchor(ANCHOR_INTERNAL_FUNCTION_CALL_PROLOGUE);
        self.emit_validate_instruction_count(None);
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 5, RSP, 8 * (SCRATCH_REGS + 1) as i64, None)); // alloca
        self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_SCRATCH, RSP, X86IndirectAccess::OffsetIndexShift(0, RSP, 0))); // Save original REGISTER_SCRATCH
        self.emit_ins(X86Instruction::load(OperandSize::S64, RSP, REGISTER_SCRATCH, X86IndirectAccess::OffsetIndexShift(8 * (SCRATCH_REGS + 1) as i32, RSP, 0))); // Load return address
        for (i, reg) in REGISTER_MAP.iter().skip(FIRST_SCRATCH_REG).take(SCRATCH_REGS).enumerate() {
            self.emit_ins(X86Instruction::store(OperandSize::S64, *reg, RSP, X86IndirectAccess::OffsetIndexShift(8 * (SCRATCH_REGS - i + 1) as i32, RSP, 0))); // Push SCRATCH_REG
        }
        // Push the caller's frame pointer. The code to restore it is emitted at the end of emit_internal_call().
        self.emit_ins(X86Instruction::store(OperandSize::S64, REGISTER_MAP[FRAME_PTR_REG], RSP, X86IndirectAccess::OffsetIndexShift(8, RSP, 0)));
        self.emit_ins(X86Instruction::xchg(OperandSize::S64, REGISTER_SCRATCH, RSP, Some(X86IndirectAccess::OffsetIndexShift(0, RSP, 0)))); // Push return address and restore original REGISTER_SCRATCH
        // Increase env.call_depth
        let call_depth_access = X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::CallDepth));
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, REGISTER_PTR_TO_VM, 1, Some(call_depth_access))); // env.call_depth += 1;
        // If env.call_depth == self.config.max_call_depth, throw CallDepthExceeded
        self.emit_ins(X86Instruction::cmp_immediate(OperandSize::S32, REGISTER_PTR_TO_VM, self.config.max_call_depth as i64, Some(call_depth_access)));
        self.emit_ins(X86Instruction::conditional_jump_immediate(0x83, self.relative_to_anchor(ANCHOR_CALL_DEPTH_EXCEEDED, 6)));
        // Setup the frame pointer for the new frame. What we do depends on whether we're using dynamic or fixed frames.
        if !self.executable.get_sbpf_version().dynamic_stack_frames() {
            // With fixed frames we start the new frame at the next fixed offset
            let stack_frame_size = self.config.stack_frame_size as i64 * if self.config.enable_stack_frame_gaps { 2 } else { 1 };
            self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, REGISTER_MAP[FRAME_PTR_REG], stack_frame_size, None)); // REGISTER_MAP[FRAME_PTR_REG] += stack_frame_size;
        }
        self.emit_ins(X86Instruction::return_near());

        // Routine for emit_internal_call(Value::Register())
        // Inputs: Guest current pc in X86IndirectAccess::OffsetIndexShift(-16, RSP, 0), Guest target address in REGISTER_SCRATCH
        // Outputs: Guest current pc in X86IndirectAccess::OffsetIndexShift(-16, RSP, 0), Guest target pc in REGISTER_SCRATCH, Host target address in RIP
        self.set_anchor(ANCHOR_INTERNAL_FUNCTION_CALL_REG);
        self.emit_ins(X86Instruction::push(REGISTER_MAP[0], None));
        // Calculate offset relative to program_vm_addr
        self.emit_ins(X86Instruction::load_immediate(REGISTER_MAP[0], self.program_vm_addr as i64));
        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x29, REGISTER_MAP[0], REGISTER_SCRATCH, None)); // guest_target_pc = guest_target_address - self.program_vm_addr;
        // Force alignment of guest_target_pc
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 4, REGISTER_SCRATCH, !(INSN_SIZE as i64 - 1), None)); // guest_target_pc &= !(INSN_SIZE - 1);
        // Bound check
        // if(guest_target_pc >= number_of_instructions * INSN_SIZE) throw CALL_OUTSIDE_TEXT_SEGMENT;
        let number_of_instructions = self.result.pc_section.len();
        self.emit_ins(X86Instruction::cmp_immediate(OperandSize::S64, REGISTER_SCRATCH, (number_of_instructions * INSN_SIZE) as i64, None)); // guest_target_pc.cmp(number_of_instructions * INSN_SIZE)
        self.emit_ins(X86Instruction::conditional_jump_immediate(0x83, self.relative_to_anchor(ANCHOR_CALL_OUTSIDE_TEXT_SEGMENT, 6)));
        // Calculate the guest_target_pc (dst / INSN_SIZE) to update REGISTER_INSTRUCTION_METER
        // and as target_pc for potential ANCHOR_CALL_REG_UNSUPPORTED_INSTRUCTION
        let shift_amount = INSN_SIZE.trailing_zeros();
        debug_assert_eq!(INSN_SIZE, 1 << shift_amount);
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0xc1, 5, REGISTER_SCRATCH, shift_amount as i64, None)); // guest_target_pc /= INSN_SIZE;
        // Load host_target_address offset from self.result.pc_section
        self.emit_ins(X86Instruction::load_immediate(REGISTER_MAP[0], self.result.pc_section.as_ptr() as i64)); // host_target_address = self.result.pc_section;
        self.emit_ins(X86Instruction::load(OperandSize::S32, REGISTER_MAP[0], REGISTER_MAP[0], X86IndirectAccess::OffsetIndexShift(0, REGISTER_SCRATCH, 2))); // host_target_address = self.result.pc_section[guest_target_pc];
        // Check destination is valid
        self.emit_ins(X86Instruction::test_immediate(OperandSize::S32, REGISTER_MAP[0], 1 << 31, None)); // host_target_address & (1 << 31)
        self.emit_ins(X86Instruction::conditional_jump_immediate(0x85, self.relative_to_anchor(ANCHOR_CALL_REG_UNSUPPORTED_INSTRUCTION, 6))); // If host_target_address & (1 << 31) != 0, throw UnsupportedInstruction
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S32, 0x81, 4, REGISTER_MAP[0], i32::MAX as i64, None)); // host_target_address &= (1 << 31) - 1;
        // A version of `self.emit_profile_instruction_count(None);` which reads self.pc from the stack
        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x2b, REGISTER_INSTRUCTION_METER, RSP, Some(X86IndirectAccess::OffsetIndexShift(-8, RSP, 0)))); // instruction_meter -= guest_current_pc;
        self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 5, REGISTER_INSTRUCTION_METER, 1, None)); // instruction_meter -= 1;
        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, REGISTER_SCRATCH, REGISTER_INSTRUCTION_METER, None)); // instruction_meter += guest_target_pc;
        // Offset host_target_address by self.result.text_section
        self.emit_ins(X86Instruction::mov_mmx(OperandSize::S64, REGISTER_SCRATCH, MM0));
        self.emit_ins(X86Instruction::load_immediate(REGISTER_SCRATCH, self.result.text_section.as_ptr() as i64)); // REGISTER_SCRATCH = self.result.text_section;
        self.emit_ins(X86Instruction::alu(OperandSize::S64, 0x01, REGISTER_SCRATCH, REGISTER_MAP[0], None)); // host_target_address += self.result.text_section;
        self.emit_ins(X86Instruction::mov_mmx(OperandSize::S64, MM0, REGISTER_SCRATCH));
        // Restore the clobbered REGISTER_MAP[0]
        self.emit_ins(X86Instruction::xchg(OperandSize::S64, REGISTER_MAP[0], RSP, Some(X86IndirectAccess::OffsetIndexShift(0, RSP, 0)))); // Swap REGISTER_MAP[0] and host_target_address
        self.emit_ins(X86Instruction::return_near()); // Tail call to host_target_address

        // Translates a vm memory address to a host memory address
        let lower_key = self.immediate_value_key as i32 as i64;
        for (anchor_base, len) in &[
            (0, 1i32), (0, 2i32), (0, 4i32), (0, 8i32),
            (4, 1i32), (4, 2i32), (4, 4i32), (4, 8i32),
            (8, 1i32), (8, 2i32), (8, 4i32), (8, 8i32),
        ] {
            let target_offset = *anchor_base + len.trailing_zeros() as usize;
            self.set_anchor(ANCHOR_TRANSLATE_MEMORY_ADDRESS + target_offset);
            // call MemoryMapping::(load|store) storing the result in RuntimeEnvironmentSlot::ProgramResult
            if *anchor_base == 0 { // AccessType::Load
                let load = match len {
                    1 => MemoryMapping::load::<u8> as *const u8 as i64,
                    2 => MemoryMapping::load::<u16> as *const u8 as i64,
                    4 => MemoryMapping::load::<u32> as *const u8 as i64,
                    8 => MemoryMapping::load::<u64> as *const u8 as i64,
                    _ => unreachable!()
                };
                self.emit_rust_call(Value::Constant64(load, false), &[
                    Argument { index: 2, value: Value::Register(REGISTER_SCRATCH) }, // Specify first as the src register could be overwritten by other arguments
                    Argument { index: 3, value: Value::Constant64(0, false) }, // self.pc is set later
                    Argument { index: 1, value: Value::RegisterPlusConstant32(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::MemoryMapping), false) },
                    Argument { index: 0, value: Value::RegisterPlusConstant32(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult), false) },
                ], None);
            } else { // AccessType::Store
                if *anchor_base == 8 {
                    // Second half of emit_sanitized_load_immediate(stack_slot_of_value_to_store, constant)
                    self.emit_ins(X86Instruction::alu_immediate(OperandSize::S64, 0x81, 0, RSP, lower_key, Some(X86IndirectAccess::OffsetIndexShift(-80, RSP, 0))));
                }
                let store = match len {
                    1 => MemoryMapping::store::<u8> as *const u8 as i64,
                    2 => MemoryMapping::store::<u16> as *const u8 as i64,
                    4 => MemoryMapping::store::<u32> as *const u8 as i64,
                    8 => MemoryMapping::store::<u64> as *const u8 as i64,
                    _ => unreachable!()
                };
                self.emit_rust_call(Value::Constant64(store, false), &[
                    Argument { index: 3, value: Value::Register(REGISTER_SCRATCH) }, // Specify first as the src register could be overwritten by other arguments
                    Argument { index: 2, value: Value::RegisterIndirect(RSP, -8, false) },
                    Argument { index: 4, value: Value::Constant64(0, false) }, // self.pc is set later
                    Argument { index: 1, value: Value::RegisterPlusConstant32(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::MemoryMapping), false) },
                    Argument { index: 0, value: Value::RegisterPlusConstant32(REGISTER_PTR_TO_VM, self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult), false) },
                ], None);
            }

            // Throw error if the result indicates one
            self.emit_result_is_err(REGISTER_SCRATCH);
            self.emit_ins(X86Instruction::pop(REGISTER_SCRATCH)); // REGISTER_SCRATCH = self.pc
            self.emit_ins(X86Instruction::xchg(OperandSize::S64, REGISTER_SCRATCH, RSP, Some(X86IndirectAccess::OffsetIndexShift(0, RSP, 0)))); // Swap return address and self.pc
            self.emit_ins(X86Instruction::conditional_jump_immediate(0x85, self.relative_to_anchor(ANCHOR_THROW_EXCEPTION, 6)));

            if *anchor_base == 0 { // AccessType::Load
                // unwrap() the result into REGISTER_SCRATCH
                self.emit_ins(X86Instruction::load(OperandSize::S64, REGISTER_PTR_TO_VM, REGISTER_SCRATCH, X86IndirectAccess::Offset(self.slot_in_vm(RuntimeEnvironmentSlot::ProgramResult) + std::mem::size_of::<u64>() as i32)));
            }

            self.emit_ins(X86Instruction::return_near());
        }
    }

    fn set_anchor(&mut self, anchor: usize) {
        self.anchors[anchor] = unsafe { self.result.text_section.as_ptr().add(self.offset_in_text_section) };
    }

    // instruction_length = 5 (Unconditional jump / call)
    // instruction_length = 6 (Conditional jump)
    fn relative_to_anchor(&self, anchor: usize, instruction_length: usize) -> i32 {
        let instruction_end = unsafe { self.result.text_section.as_ptr().add(self.offset_in_text_section).add(instruction_length) };
        let destination = self.anchors[anchor];
        debug_assert!(!destination.is_null());
        (unsafe { destination.offset_from(instruction_end) } as i32) // Relative jump
    }

    fn relative_to_target_pc(&mut self, target_pc: usize, instruction_length: usize) -> i32 {
        let instruction_end = unsafe { self.result.text_section.as_ptr().add(self.offset_in_text_section).add(instruction_length) };
        let destination = if self.result.pc_section[target_pc] != 0 {
            // Backward jump
            &self.result.text_section[self.result.pc_section[target_pc] as usize & (i32::MAX as u32 as usize)] as *const u8
        } else {
            // Forward jump, needs relocation
            self.text_section_jumps.push(Jump { location: unsafe { instruction_end.sub(4) }, target_pc });
            return 0;
        };
        debug_assert!(!destination.is_null());
        (unsafe { destination.offset_from(instruction_end) } as i32) // Relative jump
    }

    fn resolve_jumps(&mut self) {
        // Relocate forward jumps
        for jump in &self.text_section_jumps {
            let destination = &self.result.text_section[self.result.pc_section[jump.target_pc] as usize & (i32::MAX as u32 as usize)] as *const u8;
            let offset_value = 
                unsafe { destination.offset_from(jump.location) } as i32 // Relative jump
                - mem::size_of::<i32>() as i32; // Jump from end of instruction
            unsafe { ptr::write_unaligned(jump.location as *mut i32, offset_value); }
        }
    }
}
