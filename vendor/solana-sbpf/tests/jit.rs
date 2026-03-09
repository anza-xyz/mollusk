#![allow(clippy::literal_string_with_formatting_args)]
#![cfg(all(test, target_arch = "x86_64", not(target_os = "windows")))]

use byteorder::{ByteOrder, LittleEndian};
use solana_sbpf::{
    disassembler::disassemble_instruction,
    ebpf,
    elf::Executable,
    error::EbpfError,
    jit::{
        MACHINE_CODE_PER_INSTRUCTION_METER_CHECKPOINT, MAX_EMPTY_PROGRAM_MACHINE_CODE_LENGTH,
        MAX_MACHINE_CODE_LENGTH_PER_INSTRUCTION,
    },
    program::{BuiltinProgram, FunctionRegistry, SBPFVersion},
    static_analysis::CfgNode,
    vm::Config,
};
use std::{collections::BTreeMap, sync::Arc};
use test_utils::{syscalls, TestContextObject};

fn create_mockup_executable(config: Config, program: &[u8]) -> Executable<TestContextObject> {
    let sbpf_version = *config.enabled_sbpf_versions.end();
    let mut loader = BuiltinProgram::new_loader(config);
    loader
        .register_function("gather_bytes", syscalls::SyscallGatherBytes::vm)
        .unwrap();
    let mut function_registry = FunctionRegistry::default();
    function_registry
        .register_function(8, *b"function_foo", 8)
        .unwrap();
    Executable::<TestContextObject>::from_text_bytes(
        program,
        Arc::new(loader),
        sbpf_version,
        function_registry,
    )
    .unwrap()
}

#[test]
fn test_code_length_estimate() {
    const INSTRUCTION_COUNT: usize = 256;
    let mut prog = vec![0; ebpf::INSN_SIZE * INSTRUCTION_COUNT];
    for pc in 0..INSTRUCTION_COUNT {
        prog[pc * ebpf::INSN_SIZE] = ebpf::ADD64_IMM;
    }

    let mut empty_program_machine_code_length_per_version = [0; 4];
    for sbpf_version in [SBPFVersion::V0, SBPFVersion::V3] {
        let empty_program_machine_code_length = {
            let config = Config {
                noop_instruction_rate: 0,
                enabled_sbpf_versions: sbpf_version..=sbpf_version,
                ..Config::default()
            };
            let mut executable = create_mockup_executable(config, &prog[0..0]);
            Executable::<TestContextObject>::jit_compile(&mut executable).unwrap();
            executable
                .get_compiled_program()
                .unwrap()
                .machine_code_length()
        };
        assert!(empty_program_machine_code_length <= MAX_EMPTY_PROGRAM_MACHINE_CODE_LENGTH);
        empty_program_machine_code_length_per_version[sbpf_version as usize] =
            empty_program_machine_code_length;
    }

    let mut instruction_meter_checkpoint_machine_code_length = [0; 2];
    for (index, machine_code_length) in instruction_meter_checkpoint_machine_code_length
        .iter_mut()
        .enumerate()
    {
        let config = Config {
            instruction_meter_checkpoint_distance: index * INSTRUCTION_COUNT * 2,
            noop_instruction_rate: 0,
            enabled_sbpf_versions: SBPFVersion::V0..=SBPFVersion::V0,
            ..Config::default()
        };
        let mut executable = create_mockup_executable(config, &prog);
        Executable::<TestContextObject>::jit_compile(&mut executable).unwrap();
        *machine_code_length = (executable
            .get_compiled_program()
            .unwrap()
            .machine_code_length()
            - empty_program_machine_code_length_per_version[0])
            / INSTRUCTION_COUNT;
    }
    let instruction_meter_checkpoint_machine_code_length =
        instruction_meter_checkpoint_machine_code_length[0]
            - instruction_meter_checkpoint_machine_code_length[1];
    assert!(
        instruction_meter_checkpoint_machine_code_length
            <= MACHINE_CODE_PER_INSTRUCTION_METER_CHECKPOINT
    );

    let mut cfg_nodes = BTreeMap::new();
    cfg_nodes.insert(
        8,
        CfgNode {
            label: std::string::String::from("label"),
            ..CfgNode::default()
        },
    );

    for sbpf_version in [SBPFVersion::V0, SBPFVersion::V3] {
        println!("opcode;machine_code_length_per_instruction;assembly");
        let empty_program_machine_code_length =
            empty_program_machine_code_length_per_version[sbpf_version as usize];

        for mut opcode in 0x00..=0xFF {
            let (registers, immediate) = match opcode {
                0x85 if !sbpf_version.static_syscalls() => (0x00, Some(8)),
                0x85 if sbpf_version.static_syscalls() => (0x00, None),
                0x8D => (0x88, Some(0)),
                0x95 if sbpf_version.static_syscalls() => (0x00, Some(0x91020CDD)),
                0xE5 if !sbpf_version.static_syscalls() => {
                    // Put external function calls on a separate loop iteration
                    opcode = 0x85;
                    (0x00, Some(0x91020CDD))
                }
                0xF5 if !sbpf_version.static_syscalls() => {
                    // Put invalid function calls on a separate loop iteration
                    opcode = 0x85;
                    (0x00, Some(0x91020CD0))
                }
                0xD4 | 0xDC => (0x88, Some(16)),
                _ => (0x88, Some(0x11223344)),
            };
            for pc in 0..INSTRUCTION_COUNT {
                prog[pc * ebpf::INSN_SIZE] = opcode;
                prog[pc * ebpf::INSN_SIZE + 1] = registers;
                let offset = 7_u16.wrapping_sub(pc as u16);
                LittleEndian::write_u16(&mut prog[pc * ebpf::INSN_SIZE + 2..], offset);
                let immediate = immediate.unwrap_or_else(|| 7_u32.wrapping_sub(pc as u32));
                LittleEndian::write_u32(&mut prog[pc * ebpf::INSN_SIZE + 4..], immediate);
            }
            let config = Config {
                noop_instruction_rate: 0,
                enabled_sbpf_versions: sbpf_version..=sbpf_version,
                ..Config::default()
            };
            let mut executable = create_mockup_executable(config, &prog);
            let result = Executable::<TestContextObject>::jit_compile(&mut executable);
            if let Err(err) = result {
                assert!(matches!(err, EbpfError::UnsupportedInstruction));
                continue;
            }
            let machine_code_length = executable
                .get_compiled_program()
                .unwrap()
                .machine_code_length()
                - empty_program_machine_code_length;
            let instruction_count = if opcode == 0x18 {
                // LDDW takes two slots
                INSTRUCTION_COUNT / 2
            } else {
                INSTRUCTION_COUNT
            };
            let machine_code_length_per_instruction =
                machine_code_length as f64 / instruction_count as f64;
            assert!(
                f64::ceil(machine_code_length_per_instruction) as usize
                    <= MAX_MACHINE_CODE_LENGTH_PER_INSTRUCTION
            );
            let insn = ebpf::get_insn_unchecked(&prog, 0);
            let assembly = disassemble_instruction(
                &insn,
                0,
                &cfg_nodes,
                executable.get_function_registry(),
                executable.get_loader(),
                executable.get_sbpf_version(),
            );
            println!("{opcode:02X};{machine_code_length_per_instruction:>7.3};{assembly}");
        }
    }
}
