#![allow(clippy::literal_string_with_formatting_args)]
#![allow(clippy::arithmetic_side_effects)]
#![cfg(all(feature = "jit", not(target_os = "windows"), target_arch = "x86_64"))]

// Copyright 2020 Solana Maintainers <maintainers@solana.com>
//
// Licensed under the Apache License, Version 2.0 <http://www.apache.org/licenses/LICENSE-2.0> or
// the MIT license <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate byteorder;
extern crate libc;
extern crate solana_sbpf;
extern crate test_utils;
extern crate thiserror;

use rand::{rngs::SmallRng, RngCore, SeedableRng};
use solana_sbpf::{
    assembler::assemble,
    ebpf,
    memory_region::MemoryRegion,
    program::{BuiltinProgram, SBPFVersion},
    static_analysis::Analysis,
    verifier::RequisiteVerifier,
    vm::{Config, ContextObject},
};
use std::sync::Arc;
use test_utils::{compare_register_trace, create_vm, test_interpreter_and_jit, TestContextObject};

// BPF_ALU32_LOAD : Arithmetic and Logic
#[test]
fn fuzz_alu() {
    let seed = 0xC2DB2F8F282284A0;
    let mut prng = SmallRng::seed_from_u64(seed);

    for src in 0..10 {
        for dst in 0..10 {
            for _ in 0..10 {
                test_ins(false, format!("mov64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("add64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("sub64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("or64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("xor64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("and64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("lmul64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("uhmul64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("shmul64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("udiv64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("urem64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("srem64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("sdiv64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("udiv64 r{src}, r{dst}"), &mut prng, None);

                test_ins(false, format!("lsh64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("rsh64 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("arsh64 r{src}, r{dst}"), &mut prng, None);

                test_ins(false, format!("mov32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("add32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("sub32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("or32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("xor32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("and32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("lmul32 r{src}, r{dst}"), &mut prng, None);
                // test_ins(format!("uhmul32 r{src}, r{dst}"), &mut p, None);
                // test_ins(format!("shmul32 r{src}, r{dst}"), &mut p, None);
                test_ins(false, format!("udiv32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("sdiv32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("srem32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("urem32 r{src}, r{dst}"), &mut prng, None);

                test_ins(false, format!("lsh32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("rsh32 r{src}, r{dst}"), &mut prng, None);
                test_ins(false, format!("arsh32 r{src}, r{dst}"), &mut prng, None);

                test_ins(true, format!("mul64 r{src}, r{dst}"), &mut prng, None);
                test_ins(true, format!("mod64 r{src}, r{dst}"), &mut prng, None);
                test_ins(true, format!("div64 r{src}, r{dst}"), &mut prng, None);
                test_ins(true, format!("mul32 r{src}, r{dst}"), &mut prng, None);
                test_ins(true, format!("mod32 r{src}, r{dst}"), &mut prng, None);
                test_ins(true, format!("div32 r{src}, r{dst}"), &mut prng, None);

                // test load, store
                let rand = prng.next_u32() as i64;
                let offset = prng.next_u32() as i16;
                let addr = rand % 80 + 0x4_0000_0000i64 - offset as i64;
                let mut tmp = (src + 1) % 10;
                if dst == tmp {
                    tmp = (src + 2) % 10;
                }

                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         ldxb r{dst}, [r{src}{offset:+}]",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );

                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         stxb [r{src}{offset:+}], r{dst}",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );

                let addr = rand % 79 + 0x4_0000_0000i64 - offset as i64;

                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         ldxh r{dst}, [r{src}{offset:+}]",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );

                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         stxh [r{src}{offset:+}], r{dst}",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );

                let addr = rand % 77 + 0x4_0000_0000i64 - offset as i64;
                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         ldxw r{dst}, [r{src}{offset:+}]",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );
                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         stxw [r{src}{offset:+}], r{dst}",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );

                let addr = rand % 73 + 0x4_0000_0000i64 - offset as i64;

                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         ldxdw r{dst}, [r{src}{offset:+}]",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );
                test_ins(
                    false,
                    format!(
                        "mov32 r{tmp},{}
                         mov64 r{src},{:#x}
                         lsh64 r{src},32
                         or64 r{src},r{tmp}
                         stxdw [r{src}{offset:+}], r{dst}",
                        addr as i32,
                        addr >> 32,
                    ),
                    &mut prng,
                    None,
                );

                // test conditionals
                for jc in [
                    "jeq", "jgt", "jge", "jlt", "jle", "jset", "jne", "jsgt", "jsge", "jslt",
                    "jsle",
                ] {
                    test_ins(
                        false,
                        format!(
                            "{jc} r{src}, r{dst}, l1
                            or64 r{src},0x12345678
                            ja l2
                        l1:
                            and64 r{dst},0x12345678
                            ja l2
                        l2:",
                        ),
                        &mut prng,
                        Some(2),
                    );
                }
            }
        }

        for _ in 0..10 {
            let imm = prng.next_u64() as i64;
            test_ins(true, format!("lddw r{src}, {imm}"), &mut prng, Some(0));

            let mut imm = imm as i32;

            test_ins(true, format!("neg64 r{src}"), &mut prng, None);
            test_ins(true, format!("neg32 r{src}"), &mut prng, None);

            test_ins(true, format!("mul64 r{src}, {imm}"), &mut prng, None);
            test_ins(true, format!("mod64 r{src}, {imm}"), &mut prng, None);
            test_ins(true, format!("div64 r{src}, {imm}"), &mut prng, None);

            test_ins(true, format!("mul32 r{src}, {imm}"), &mut prng, None);
            test_ins(true, format!("mod32 r{src}, {imm}"), &mut prng, None);
            test_ins(true, format!("div32 r{src}, {imm}"), &mut prng, None);

            test_ins(false, format!("mov64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("add64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("sub64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("or64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("xor64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("and64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("lmul64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("uhmul64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("shmul64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("udiv64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("urem64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("sdiv64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("srem64 r{src}, {imm}"), &mut prng, None);

            test_ins(false, format!("mov32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("add32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("sub32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("or32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("xor32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("and32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("lmul32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("udiv32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("urem32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("sdiv32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("srem32 r{src}, {imm}"), &mut prng, None);

            // test st imm
            let rand = prng.next_u32() as i64;
            let offset = prng.next_u32() as i16;
            let tmp = (src + 1) % 10;

            let addr = rand % 80 + 0x4_0000_0000i64 - offset as i64;
            test_ins(
                false,
                format!(
                    "mov32 r{tmp},{}
                    mov64 r{src},{:#x}
                    lsh64 r{src},32
                    or64 r{src},r{tmp}
                    stb [r{src}{offset:+}], {}",
                    addr as i32,
                    addr >> 32,
                    imm as i8,
                ),
                &mut prng,
                None,
            );

            let addr = rand % 79 + 0x4_0000_0000i64 - offset as i64;
            test_ins(
                false,
                format!(
                    "mov32 r{tmp},{}
                    mov64 r{src},{:#x}
                    lsh64 r{src},32
                    or64 r{src},r{tmp}
                    sth [r{src}{offset:+}], {}",
                    addr as i32,
                    addr >> 32,
                    imm as i16
                ),
                &mut prng,
                None,
            );

            let addr = rand % 77 + 0x4_0000_0000i64 - offset as i64;
            test_ins(
                false,
                format!(
                    "mov32 r{tmp},{}
                    mov64 r{src},{:#x}
                    lsh64 r{src},32
                    or64 r{src},r{tmp}
                    stw [r{src}{offset:+}], {imm}",
                    addr as i32,
                    addr >> 32,
                ),
                &mut prng,
                None,
            );

            let addr = rand % 73 + 0x4_0000_0000i64 - offset as i64;
            test_ins(
                false,
                format!(
                    "mov32 r{tmp},{}
                    mov64 r{src},{:#x}
                    lsh64 r{src},32
                    or64 r{src},r{tmp}
                    stdw [r{src}{offset:+}], {imm}",
                    addr as i32,
                    addr >> 32,
                ),
                &mut prng,
                None,
            );

            // unconditional jump
            test_ins(
                false,
                format!(
                    "ja 1
                     xor64 r{src},0x12345678
            1:",
                ),
                &mut prng,
                Some(0),
            );

            for jc in [
                "jeq", "jgt", "jge", "jlt", "jle", "jset", "jne", "jsgt", "jsge", "jslt", "jsle",
            ] {
                test_ins(
                    false,
                    format!(
                        "{jc} r{src}, {imm}, l1
                        or64 r{src},0x12345678
                        ja l2
                    l1:
                        and64 r{src},0x12345678
                        ja l2
                    l2:",
                    ),
                    &mut prng,
                    Some(2),
                );
            }

            imm &= 63;

            test_ins(false, format!("lsh64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("rsh64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("arsh64 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("hor64 r{src}, {imm}"), &mut prng, None);

            imm &= 31;

            test_ins(false, format!("lsh32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("rsh32 r{src}, {imm}"), &mut prng, None);
            test_ins(false, format!("arsh32 r{src}, {imm}"), &mut prng, None);

            test_ins(false, format!("be64 r{src}"), &mut prng, None);
            test_ins(false, format!("be32 r{src}"), &mut prng, None);
            test_ins(false, format!("be16 r{src}"), &mut prng, None);

            test_ins(true, format!("le64 r{src}"), &mut prng, None);
            test_ins(true, format!("le32 r{src}"), &mut prng, None);
            test_ins(true, format!("le16 r{src}"), &mut prng, None);
        }
    }
}

fn test_ins(v0: bool, ins: String, prng: &mut SmallRng, cu: Option<u64>) {
    let mut input = [0u8; 80];

    prng.fill_bytes(&mut input);

    let asm = format!(
        "
        add64 r10, 0
        ldxdw r9, [r1+72]
        ldxdw r8, [r1+64]
        ldxdw r7, [r1+56]
        ldxdw r6, [r1+48]
        ldxdw r5, [r1+40]
        ldxdw r4, [r1+32]
        ldxdw r3, [r1+24]
        ldxdw r2, [r1+16]
        ldxdw r0, [r1+0]
        ldxdw r1, [r1+8]
        {ins}
        xor64 r0, r1
        xor64 r0, r2
        xor64 r0, r3
        xor64 r0, r4
        xor64 r0, r5
        xor64 r0, r6
        xor64 r0, r7
        xor64 r0, r8
        xor64 r0, r9
        exit"
    );

    let mut config = Config {
        enable_register_tracing: true,
        ..Config::default()
    };
    if v0 {
        config.enabled_sbpf_versions = SBPFVersion::V0..=SBPFVersion::V0;
    }
    let loader = Arc::new(BuiltinProgram::new_loader(config));
    let mut executable = assemble(asm.as_str(), loader).unwrap();
    let cu = cu
        .map(|cu| cu.saturating_add(22))
        .unwrap_or(executable.get_text_bytes().1.len().wrapping_shr(3) as u64);
    test_interpreter_and_jit!(
        override_budget => true,
        executable,
        input,
        TestContextObject::new(cu),
    );
}
