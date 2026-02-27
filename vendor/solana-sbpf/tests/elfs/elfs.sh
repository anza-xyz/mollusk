#!/bin/bash -ex

# Requires Latest release of Solana's custom LLVM
# https://github.com/anza-xyz/platform-tools/releases

TOOLCHAIN="$HOME"/.cache/solana/v1.50/platform-tools/

CC_V0="$TOOLCHAIN/llvm/bin/clang -Werror -target sbf -O2 -fno-builtin -fPIC"
CC_V4="$CC_V0 -mcpu=v4"

RC_V0="$TOOLCHAIN/rust/bin/rustc --target sbf-solana-solana --crate-type lib -C panic=abort -C opt-level=2"
RC_V4="$RC_V0 -C target_cpu=v4 -C target_feature=+static-syscalls,+abi-v2"

LD_COMMON="$TOOLCHAIN/llvm/bin/ld.lld -z notext -shared --Bdynamic -entry entrypoint"
LD_V4="$LD_COMMON -Bsymbolic --script elf.ld"
LD_V0="$LD_COMMON --script elf_sbpfv0.ld"

$RC_V4 -o strict_header.o strict_header.rs
$LD_V4 -o strict_header.so strict_header.o

$RC_V0 -o relative_call.o relative_call.rs
$LD_V0 -o relative_call_sbpfv0.so relative_call.o

$RC_V4 -o relative_call.o relative_call.rs
$LD_V4 -o relative_call.so relative_call.o

$RC_V0 -o syscall_reloc_64_32.o syscall_reloc_64_32.rs
$LD_V0 -o syscall_reloc_64_32_sbpfv0.so syscall_reloc_64_32.o

$RC_V0 -o bss_section.o bss_section.rs
$LD_V0 -o bss_section_sbpfv0.so bss_section.o

$RC_V0 -o data_section.o data_section.rs
$LD_V0 -o data_section_sbpfv0.so data_section.o

$RC_V0 -o rodata_section.o rodata_section.rs
$LD_V0 -o rodata_section_sbpfv0.so rodata_section.o

$RC_V4 -o rodata_section.o rodata_section.rs
$LD_V4 -o rodata_section.so rodata_section.o

$RC_V0 -o program_headers_overflow.o rodata_section.rs
"$TOOLCHAIN"/llvm/bin/ld.lld -z notext -shared --Bdynamic -entry entrypoint --script program_headers_overflow.ld --noinhibit-exec -o program_headers_overflow.so program_headers_overflow.o

$RC_V0 -o struct_func_pointer.o struct_func_pointer.rs
$LD_V0 -o struct_func_pointer_sbpfv0.so struct_func_pointer.o

$RC_V4 -o struct_func_pointer.o struct_func_pointer.rs
$LD_V4 -o struct_func_pointer.so struct_func_pointer.o

$RC_V0 -o reloc_64_64.o reloc_64_64.rs
$LD_V0 -o reloc_64_64_sbpfv0.so reloc_64_64.o

$RC_V4 -o reloc_64_64.o reloc_64_64.rs
$LD_V4 -o reloc_64_64.so reloc_64_64.o

$RC_V4 -o reloc_64_64.o reloc_64_64.rs
$LD_V4 -o reloc_64_64.so reloc_64_64.o

$RC_V0 -o reloc_64_relative.o reloc_64_relative.rs
$LD_V0 -o reloc_64_relative_sbpfv0.so reloc_64_relative.o

$RC_V4 -o reloc_64_relative.o reloc_64_relative.rs
$LD_V4 -o reloc_64_relative.so reloc_64_relative.o

$CC_V0 -o reloc_64_relative_data.o -c reloc_64_relative_data.c
$LD_V0 -o reloc_64_relative_data_sbpfv0.so reloc_64_relative_data.o

$CC_V4 -o reloc_64_relative_data.o -c reloc_64_relative_data.c
$LD_V4 -o reloc_64_relative_data.so reloc_64_relative_data.o

$RC_V4 -o syscall_static.o syscall_static.rs
$LD_V4 -o syscall_static.so syscall_static.o

# $RC_V1 -o callx_unaligned.o callx_unaligned.rs
# $LD_V1 -o callx_unaligned.so callx_unaligned.o

rm *.o
