#![allow(clippy::literal_string_with_formatting_args)]

// Converted from the tests for uBPF <https://github.com/iovisor/ubpf>
// Copyright 2015 Big Switch Networks, Inc
// Copyright 2016 6WIND S.A. <quentin.monnet@6wind.com>
//
// Licensed under the Apache License, Version 2.0 <http://www.apache.org/licenses/LICENSE-2.0> or
// the MIT license <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// The tests contained in this file are extracted from the unit tests of uBPF software. Each test
// in this file has a name in the form `test_verifier_<name>`, and corresponds to the
// (human-readable) code in `ubpf/tree/master/tests/<name>`, available at
// <https://github.com/iovisor/ubpf/tree/master/tests> (hyphen had to be replaced with underscores
// as Rust will not accept them in function names). It is strongly advised to refer to the uBPF
// version to understand what these program do.
//
// Each program was assembled from the uBPF version with the assembler provided by uBPF itself, and
// available at <https://github.com/iovisor/ubpf/tree/master/ubpf>.
// The very few modifications that have been realized should be indicated.

// These are unit tests for the eBPF “verifier”.

extern crate solana_sbpf;
extern crate thiserror;

use solana_sbpf::{
    assembler::assemble,
    ebpf,
    elf::Executable,
    program::{BuiltinFunction, BuiltinProgram, FunctionRegistry, SBPFVersion},
    verifier::{RequisiteVerifier, Verifier, VerifierError},
    vm::{Config, ContextObject},
};
use std::sync::Arc;
use test_utils::{assert_error, create_vm, syscalls, TestContextObject};
use thiserror::Error;

/// Error definitions
#[derive(Debug, Error)]
pub enum VerifierTestError {
    #[error("{0}")]
    Rejected(String),
}

struct TautologyVerifier {}
impl Verifier for TautologyVerifier {
    fn verify<C: ContextObject>(
        _prog: &[u8],
        _config: &Config,
        _sbpf_version: SBPFVersion,
        _function_registry: &FunctionRegistry<usize>,
        _syscall_registry: &FunctionRegistry<BuiltinFunction<C>>,
    ) -> std::result::Result<(), VerifierError> {
        Ok(())
    }
}

struct ContradictionVerifier {}
impl Verifier for ContradictionVerifier {
    fn verify<C: ContextObject>(
        _prog: &[u8],
        _config: &Config,
        _sbpf_version: SBPFVersion,
        _function_registry: &FunctionRegistry<usize>,
        _syscall_registry: &FunctionRegistry<BuiltinFunction<C>>,
    ) -> std::result::Result<(), VerifierError> {
        Err(VerifierError::NoProgram)
    }
}

#[test]
fn test_verifier_success() {
    let executable = assemble::<TestContextObject>(
        "
        mov32 r0, 0xBEE
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<TautologyVerifier>().unwrap();
    let mut context_object = TestContextObject::default();
    create_vm!(
        _vm,
        &executable,
        &mut context_object,
        stack,
        heap,
        Vec::new(),
        None
    );
}

#[test]
#[should_panic(expected = "NoProgram")]
fn test_verifier_fail() {
    let executable = assemble::<TestContextObject>(
        "
        mov32 r0, 0xBEE
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<ContradictionVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "DivisionByZero(2)")]
fn test_verifier_err_div_by_zero_imm() {
    let executable = assemble::<TestContextObject>(
        "
        add64 r10, 0
        mov32 r0, 1
        udiv32 r0, 0
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "UnsupportedLEBEArgument(1)")]
fn test_verifier_err_endian_size() {
    let prog = &[
        0x07, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
        0xdc, 0x01, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, //
        0xb7, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
        0x9d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
    ];
    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(BuiltinProgram::new_mock()),
        SBPFVersion::V4,
        FunctionRegistry::default(),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "IncompleteLDDW(0)")]
fn test_verifier_err_incomplete_lddw() {
    // Note: ubpf has test-err-incomplete-lddw2, which is the same
    // lddw r0, 0x55667788
    let prog = &[
        0x18, 0x00, 0x00, 0x00, 0x88, 0x77, 0x66, 0x55, //
        0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
    ];
    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(BuiltinProgram::new_mock()),
        SBPFVersion::V0,
        FunctionRegistry::default(),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "LDDWCannotBeLast")]
fn test_verifier_err_lddw_cannot_be_last() {
    for highest_sbpf_version in [SBPFVersion::V0, SBPFVersion::V4] {
        let prog = &[0x18, 0x00, 0x00, 0x00, 0x88, 0x77, 0x66, 0x55];
        let executable = Executable::<TestContextObject>::from_text_bytes(
            prog,
            Arc::new(BuiltinProgram::new_loader(Config {
                enabled_sbpf_versions: SBPFVersion::V0..=highest_sbpf_version,
                ..Config::default()
            })),
            highest_sbpf_version,
            FunctionRegistry::default(),
        )
        .unwrap();
        executable.verify::<RequisiteVerifier>().unwrap();
    }
}

#[test]
fn test_verifier_err_invalid_reg_dst() {
    // r11 is disabled when sbpf_version.dynamic_stack_frames()=false, and only sub and add are
    // allowed when sbpf_version.dynamic_stack_frames()=true
    for highest_sbpf_version in [SBPFVersion::V0, SBPFVersion::V4] {
        let executable = assemble::<TestContextObject>(
            "
            add64 r10, 0
            mov r11, 1
            exit",
            Arc::new(BuiltinProgram::new_loader(Config {
                enabled_sbpf_versions: SBPFVersion::V0..=highest_sbpf_version,
                ..Config::default()
            })),
        )
        .unwrap();
        let result = executable.verify::<RequisiteVerifier>();
        assert_error!(result, "VerifierError(InvalidDestinationRegister(1))");
    }
}

#[test]
fn test_verifier_err_invalid_reg_src() {
    // r11 is disabled when sbpf_version.dynamic_stack_frames()=false, and only sub and add are
    // allowed when sbpf_version.dynamic_stack_frames()=true
    for highest_sbpf_version in [SBPFVersion::V0, SBPFVersion::V4] {
        let executable = assemble::<TestContextObject>(
            "
            add64 r10, 0
            mov r0, r11
            exit",
            Arc::new(BuiltinProgram::new_loader(Config {
                enabled_sbpf_versions: SBPFVersion::V0..=highest_sbpf_version,
                ..Config::default()
            })),
        )
        .unwrap();
        let result = executable.verify::<RequisiteVerifier>();
        assert_error!(result, "VerifierError(InvalidSourceRegister(1))");
    }
}

#[test]
fn test_verifier_resize_stack_ptr_success() {
    let executable = assemble::<TestContextObject>(
        "
        add r10, -64
        exit
        add r10, 64
        exit",
        Arc::new(BuiltinProgram::new_loader(Config {
            enable_stack_frame_gaps: false,
            ..Config::default()
        })),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "UnalignedImmediate(0)")]
fn test_verifier_negative_unaligned_stack() {
    let executable = assemble::<TestContextObject>(
        "
        add r10, -63
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "UnalignedImmediate(0)")]
fn test_verifier_positive_unaligned_stack() {
    let executable = assemble::<TestContextObject>(
        "
        add r10, 63
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "JumpToMiddleOfLDDW(2, 0)")]
fn test_verifier_err_jmp_lddw() {
    let executable = assemble::<TestContextObject>(
        "
        ja +1
        lddw r0, 0x1122334455667788
        exit",
        Arc::new(BuiltinProgram::new_loader(Config {
            enabled_sbpf_versions: SBPFVersion::V0..=SBPFVersion::V0,
            ..Config::default()
        })),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
fn test_verifier_call_into_lddw() {
    let executable = assemble::<TestContextObject>(
        "
        call 1
        lddw r0, 0x1122334455667788
        exit",
        Arc::new(BuiltinProgram::new_loader(Config {
            enabled_sbpf_versions: SBPFVersion::V0..=SBPFVersion::V0,
            ..Config::default()
        })),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "InvalidRegister(0)")]
fn test_verifier_err_callx_cannot_use_r10() {
    for highest_sbpf_version in [SBPFVersion::V0, SBPFVersion::V4] {
        let executable = assemble::<TestContextObject>(
            "
        callx r10
        exit
        ",
            Arc::new(BuiltinProgram::new_loader(Config {
                enabled_sbpf_versions: SBPFVersion::V0..=highest_sbpf_version,
                ..Config::default()
            })),
        )
        .unwrap();
        executable.verify::<RequisiteVerifier>().unwrap();
    }
}

#[test]
#[should_panic(expected = "InvalidFunction(0)")]
fn test_verifier_err_function_fallthrough() {
    let executable = assemble::<TestContextObject>(
        "
        mov r0, r1
        function_foo:
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "JumpOutOfCode(4, 1)")]
fn test_verifier_err_jmp_out() {
    let executable = assemble::<TestContextObject>(
        "
        add64 r10, 0
        ja +2
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "JumpOutOfCode(18446744073709551615, 1)")]
fn test_verifier_err_jmp_out_start() {
    let executable = assemble::<TestContextObject>(
        "
        add64 r10, 0
        ja -3
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "UnknownOpCode(157, 0)")]
fn test_verifier_err_invalid_return() {
    let prog = &[
        0x9d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // return
    ];
    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(BuiltinProgram::new_mock()),
        SBPFVersion::V0,
        FunctionRegistry::default(),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "InvalidFunction(0)")]
fn test_verifier_err_invalid_exit() {
    let prog = &[
        0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit in v0, but syscall in v3
    ];
    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(BuiltinProgram::new_mock()),
        SBPFVersion::V4,
        FunctionRegistry::default(),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "InvalidSyscall(2432830685)")]
fn test_verifier_unknown_syscall() {
    let prog = &[
        0x07, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // add64 r10, 0
        0x95, 0x00, 0x00, 0x00, 0xDD, 0x0C, 0x02, 0x91, // syscall gather_bytes
        0x9d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // return
    ];
    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(BuiltinProgram::new_mock()),
        SBPFVersion::V4,
        FunctionRegistry::default(),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
fn test_verifier_known_syscall() {
    let prog = &[
        0x07, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // add64 r10, 0
        0x95, 0x00, 0x00, 0x00, 0xDD, 0x0C, 0x02, 0x91, // syscall gather_bytes
        0x9d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // return
    ];

    let mut loader = BuiltinProgram::new_loader(Config::default());
    loader
        .register_function("gather_bytes", syscalls::SyscallString::vm)
        .unwrap();
    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(loader),
        SBPFVersion::V4,
        FunctionRegistry::default(),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
#[should_panic(expected = "CannotWriteR10(1)")]
fn test_verifier_err_write_r10() {
    let executable = assemble::<TestContextObject>(
        "
        add64 r10, 0
        mov r10, 1
        exit",
        Arc::new(BuiltinProgram::new_mock()),
    )
    .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
}

#[test]
fn test_verifier_err_all_shift_overflows() {
    let testcases = [
        // lsh32_imm
        ("add64 r10, 0\n    lsh32 r0, 16", Ok(())),
        (
            "add64 r10, 0\n    lsh32 r0, 32",
            Err("ShiftWithOverflow(32, 32, 1)"),
        ),
        (
            "add64 r10, 0\n    lsh32 r0, 64",
            Err("ShiftWithOverflow(64, 32, 1)"),
        ),
        // rsh32_imm
        ("add64 r10, 0\n    rsh32 r0, 16", Ok(())),
        (
            "add64 r10, 0\n    rsh32 r0, 32",
            Err("ShiftWithOverflow(32, 32, 1)"),
        ),
        (
            "add64 r10, 0\n    rsh32 r0, 64",
            Err("ShiftWithOverflow(64, 32, 1)"),
        ),
        // arsh32_imm
        ("add64 r10, 0\n    arsh32 r0, 16", Ok(())),
        (
            "add64 r10, 0\n    arsh32 r0, 32",
            Err("ShiftWithOverflow(32, 32, 1)"),
        ),
        (
            "add64 r10, 0\n    arsh32 r0, 64",
            Err("ShiftWithOverflow(64, 32, 1)"),
        ),
        // lsh64_imm
        ("add64 r10, 0\n    lsh64 r0, 32", Ok(())),
        (
            "add64 r10, 0\n    lsh64 r0, 64",
            Err("ShiftWithOverflow(64, 64, 1)"),
        ),
        // rsh64_imm
        ("add64 r10, 0\n    rsh64 r0, 32", Ok(())),
        (
            "add64 r10, 0\n    rsh64 r0, 64",
            Err("ShiftWithOverflow(64, 64, 1)"),
        ),
        // arsh64_imm
        ("add64 r10, 0\n    arsh64 r0, 32", Ok(())),
        (
            "add64 r10, 0\n    arsh64 r0, 64",
            Err("ShiftWithOverflow(64, 64, 1)"),
        ),
    ];

    for (overflowing_instruction, expected) in testcases {
        let assembly = format!("\n{overflowing_instruction}\nexit");
        let executable =
            assemble::<TestContextObject>(&assembly, Arc::new(BuiltinProgram::new_mock())).unwrap();
        let result = executable.verify::<RequisiteVerifier>();
        match expected {
            Ok(()) => assert!(result.is_ok()),
            Err(overflow_msg) => assert_error!(result, "VerifierError({overflow_msg})"),
        }
    }
}

#[test]
fn test_sdiv_disabled() {
    let instructions = [
        (ebpf::SDIV32_IMM, "sdiv32 r0, 2"),
        (ebpf::SDIV32_REG, "sdiv32 r0, r1"),
        (ebpf::SDIV64_IMM, "sdiv64 r0, 4"),
        (ebpf::SDIV64_REG, "sdiv64 r0, r1"),
    ];

    for (opc, instruction) in instructions {
        for highest_sbpf_version in [SBPFVersion::V1, SBPFVersion::V4] {
            let assembly = format!("add64 r10, 0\n{instruction}\nexit");
            let executable = assemble::<TestContextObject>(
                &assembly,
                Arc::new(BuiltinProgram::new_loader(Config {
                    enabled_sbpf_versions: SBPFVersion::V0..=highest_sbpf_version,
                    ..Config::default()
                })),
            )
            .unwrap();
            let result = executable.verify::<RequisiteVerifier>();
            if highest_sbpf_version == SBPFVersion::V4 {
                assert!(result.is_ok());
            } else {
                assert_error!(result, "VerifierError(UnknownOpCode({}, {}))", opc, 1);
            }
        }
    }
}

#[test]
fn return_instr() {
    for sbpf_version in [SBPFVersion::V1, SBPFVersion::V4] {
        let prog = &[
            0x07, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // add64 r10, 0
            0xbf, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, // mov64 r0, 2
            0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit (v1), syscall (v2)
            0x9d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // return
        ];

        let executable = Executable::<TestContextObject>::from_text_bytes(
            prog,
            Arc::new(BuiltinProgram::new_mock()),
            sbpf_version,
            FunctionRegistry::default(),
        )
        .unwrap();
        let result = executable.verify::<RequisiteVerifier>();
        if sbpf_version == SBPFVersion::V4 {
            assert_error!(result, "VerifierError(InvalidSyscall(0))");
        } else {
            assert_error!(result, "VerifierError(UnknownOpCode(157, 3))");
        }
    }
}

#[test]
fn return_in_v2() {
    let executable = assemble::<TestContextObject>(
        "add64 r10, 0
        mov r0, 2
        return",
        Arc::new(BuiltinProgram::new_loader(Config {
            enabled_sbpf_versions: SBPFVersion::V3..=SBPFVersion::V4,
            ..Config::default()
        })),
    )
    .unwrap();
    let result = executable.verify::<RequisiteVerifier>();
    assert!(result.is_ok());
}

#[test]
fn function_without_return() {
    let executable = assemble::<TestContextObject>(
        "
        add64 r10, 0
        mov r0, 2
        add64 r0, 5",
        Arc::new(BuiltinProgram::new_loader(Config {
            enabled_sbpf_versions: SBPFVersion::V3..=SBPFVersion::V4,
            ..Config::default()
        })),
    )
    .unwrap();
    let result = executable.verify::<RequisiteVerifier>();
    assert_error!(result, "VerifierError(InvalidFunction(2))");
}
