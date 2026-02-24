#![allow(clippy::literal_string_with_formatting_args)]

use solana_sbpf::{
    elf::Executable,
    program::BuiltinProgram,
    vm::{Config, RuntimeEnvironmentSlot},
};
use std::{fs::File, io::Read, sync::Arc};
use test_utils::{create_vm, syscalls, TestContextObject};

#[test]
fn test_runtime_environment_slots() {
    let mut file = File::open("tests/elfs/relative_call_sbpfv0.so").unwrap();
    let mut elf = Vec::new();
    file.read_to_end(&mut elf).unwrap();
    let executable =
        Executable::<TestContextObject>::from_elf(&elf, Arc::new(BuiltinProgram::new_mock()))
            .unwrap();
    let mut context_object = TestContextObject::default();
    create_vm!(
        env,
        &executable,
        &mut context_object,
        stack,
        heap,
        Vec::new(),
        None
    );

    macro_rules! check_slot {
        ($env:expr, $entry:ident, $slot:ident) => {
            assert_eq!(
                unsafe {
                    std::ptr::addr_of!($env.$entry)
                        .cast::<u64>()
                        .offset_from(std::ptr::addr_of!($env).cast::<u64>()) as usize
                },
                RuntimeEnvironmentSlot::$slot as usize,
            );
        };
    }

    check_slot!(env, host_stack_pointer, HostStackPointer);
    check_slot!(env, call_depth, CallDepth);
    check_slot!(env, context_object_pointer, ContextObjectPointer);
    check_slot!(env, previous_instruction_meter, PreviousInstructionMeter);
    check_slot!(env, due_insn_count, DueInsnCount);
    check_slot!(env, stopwatch_numerator, StopwatchNumerator);
    check_slot!(env, stopwatch_denominator, StopwatchDenominator);
    check_slot!(env, registers, Registers);
    check_slot!(env, program_result, ProgramResult);
    check_slot!(env, memory_mapping, MemoryMapping);
    check_slot!(env, register_trace, RegisterTrace);
}

#[test]
fn test_builtin_program_eq() {
    let mut builtin_program_a = BuiltinProgram::new_loader(Config::default());
    let mut builtin_program_b = BuiltinProgram::new_loader(Config::default());
    let mut builtin_program_c = BuiltinProgram::new_loader(Config::default());
    builtin_program_a
        .register_function("log", syscalls::SyscallString::vm)
        .unwrap();
    builtin_program_a
        .register_function("log_64", syscalls::SyscallU64::vm)
        .unwrap();
    builtin_program_b
        .register_function("log_64", syscalls::SyscallU64::vm)
        .unwrap();
    builtin_program_b
        .register_function("log", syscalls::SyscallString::vm)
        .unwrap();
    builtin_program_c
        .register_function("log_64", syscalls::SyscallU64::vm)
        .unwrap();
    assert_eq!(builtin_program_a, builtin_program_b);
    assert_ne!(builtin_program_a, builtin_program_c);
}
