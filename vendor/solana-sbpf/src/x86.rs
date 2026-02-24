#![allow(clippy::arithmetic_side_effects)]
use crate::{
    jit::{JitCompiler, OperandSize},
    vm::ContextObject,
};

macro_rules! exclude_operand_sizes {
    ($size:expr, $($to_exclude:path)|+ $(,)?) => {
        debug_assert!(match $size {
            $($to_exclude)|+ => false,
            _ => true,
        });
    }
}

#[allow(dead_code, clippy::upper_case_acronyms)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum X86Register {
    RAX = 0,
    RCX = 1,
    RDX = 2,
    RBX = 3,
    RSP = 4,
    RBP = 5,
    RSI = 6,
    RDI = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
    MM0 = 16,
    MM1 = 17,
    MM2 = 18,
    MM3 = 19,
    MM4 = 20,
    MM5 = 21,
    MM6 = 22,
    MM7 = 23,
}
use X86Register::*;

// System V AMD64 ABI
// Works on: Linux, macOS, BSD and Solaris but not on Windows
pub const ARGUMENT_REGISTERS: [X86Register; 6] = [RDI, RSI, RDX, RCX, R8, R9];
pub const CALLER_SAVED_REGISTERS: [X86Register; 9] = [RAX, RCX, RDX, RSI, RDI, R8, R9, R10, R11];
pub const CALLEE_SAVED_REGISTERS: [X86Register; 6] = [RBP, RBX, R12, R13, R14, R15];

struct X86Rex {
    w: bool,
    r: bool,
    x: bool,
    b: bool,
}

struct X86ModRm {
    mode: u8,
    r: u8,
    m: u8,
}

struct X86Sib {
    scale: u8,
    index: u8,
    base: u8,
}

#[derive(Copy, Clone)]
pub enum X86IndirectAccess {
    /// [second_operand + offset]
    Offset(i32),
    /// [second_operand + offset + index << shift]
    OffsetIndexShift(i32, X86Register, u8),
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum FenceType {
    /// lfence
    Load = 5,
    /// mfence
    All = 6,
    /// sfence
    Store = 7,
}

#[derive(Copy, Clone)]
pub struct X86Instruction {
    size: OperandSize,
    opcode_escape_sequence: u8,
    opcode: u8,
    modrm: bool,
    indirect: Option<X86IndirectAccess>,
    first_operand: u8,
    second_operand: u8,
    immediate_size: OperandSize,
    immediate: i64,
}

impl X86Instruction {
    pub const DEFAULT: X86Instruction = X86Instruction {
        size: OperandSize::S0,
        opcode_escape_sequence: 0,
        opcode: 0,
        modrm: true,
        indirect: None,
        first_operand: 0,
        second_operand: 0,
        immediate_size: OperandSize::S0,
        immediate: 0,
    };

    #[inline(always)]
    pub fn emit<C: ContextObject>(&self, jit: &mut JitCompiler<C>) {
        debug_assert!(!matches!(self.size, OperandSize::S0));
        let mut rex = X86Rex {
            w: matches!(self.size, OperandSize::S64),
            r: self.first_operand & 0b1000 != 0,
            x: false,
            b: self.second_operand & 0b1000 != 0,
        };
        let mut modrm = X86ModRm {
            mode: 0,
            r: self.first_operand & 0b111,
            m: self.second_operand & 0b111,
        };
        let mut sib = X86Sib {
            scale: 0,
            index: 0,
            base: 0,
        };
        let mut displacement_size = OperandSize::S0;
        let mut displacement = 0;
        if self.modrm {
            match self.indirect {
                Some(X86IndirectAccess::Offset(offset)) => {
                    displacement = offset;
                    debug_assert_ne!(self.second_operand & 0b111, 4); // Reserved for SIB addressing
                    if (-128..=127).contains(&displacement)
                        || (displacement == 0 && self.second_operand & 0b111 == 5)
                    {
                        displacement_size = OperandSize::S8;
                        modrm.mode = 1;
                    } else {
                        displacement_size = OperandSize::S32;
                        modrm.mode = 2;
                    }
                }
                Some(X86IndirectAccess::OffsetIndexShift(offset, index, shift)) => {
                    displacement = offset;
                    if (-128..=127).contains(&displacement) {
                        displacement_size = OperandSize::S8;
                        modrm.mode = 1;
                    } else {
                        displacement_size = OperandSize::S32;
                        modrm.mode = 2;
                    }
                    modrm.m = 4;
                    rex.x = (index as u8) & 0b1000 != 0;
                    sib.scale = shift & 0b11;
                    sib.index = (index as u8) & 0b111;
                    sib.base = self.second_operand & 0b111;
                }
                None => {
                    modrm.mode = 3;
                }
            }
        }
        if matches!(self.size, OperandSize::S16) {
            jit.emit::<u8>(0x66);
        }
        let rex =
            ((rex.w as u8) << 3) | ((rex.r as u8) << 2) | ((rex.x as u8) << 1) | (rex.b as u8);
        if rex != 0 {
            jit.emit::<u8>(0x40 | rex);
        }
        match self.opcode_escape_sequence {
            1 => jit.emit::<u8>(0x0f),
            2 => jit.emit::<u16>(0x0f38),
            3 => jit.emit::<u16>(0x0f3a),
            _ => {}
        }
        jit.emit::<u8>(self.opcode);
        if self.modrm {
            jit.emit::<u8>((modrm.mode << 6) | (modrm.r << 3) | modrm.m);
            let sib = (sib.scale << 6) | (sib.index << 3) | sib.base;
            if sib != 0 {
                jit.emit::<u8>(sib);
            }
            jit.emit_variable_length(displacement_size, displacement as u64);
        }
        jit.emit_variable_length(self.immediate_size, self.immediate as u64);
    }

    /// Arithmetic or logic
    pub const fn alu_escaped(
        size: OperandSize,
        opcode_escape_sequence: u8,
        opcode: u8,
        source: X86Register,
        destination: X86Register,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8 | OperandSize::S16);
        Self {
            size,
            opcode_escape_sequence,
            opcode,
            first_operand: source as u8,
            second_operand: destination as u8,
            indirect,
            ..X86Instruction::DEFAULT
        }
    }

    /// Arithmetic or logic
    pub const fn alu_immediate_escaped(
        size: OperandSize,
        opcode_escape_sequence: u8,
        opcode: u8,
        opcode_extension: u8,
        destination: X86Register,
        immediate: i64,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8 | OperandSize::S16);
        Self {
            size,
            opcode_escape_sequence,
            opcode,
            first_operand: opcode_extension,
            second_operand: destination as u8,
            immediate_size: match opcode {
                0xc1 => OperandSize::S8,
                0x81 => OperandSize::S32,
                0xf7 if opcode_extension == 0 => OperandSize::S32,
                _ => OperandSize::S0,
            },
            immediate,
            indirect,
            ..X86Instruction::DEFAULT
        }
    }

    /// Arithmetic or logic
    pub const fn alu(
        size: OperandSize,
        opcode: u8,
        source: X86Register,
        destination: X86Register,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        Self::alu_escaped(size, 0, opcode, source, destination, indirect)
    }

    /// Arithmetic or logic
    pub const fn alu_immediate(
        size: OperandSize,
        opcode: u8,
        opcode_extension: u8,
        destination: X86Register,
        immediate: i64,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        Self::alu_immediate_escaped(
            size,
            0,
            opcode,
            opcode_extension,
            destination,
            immediate,
            indirect,
        )
    }

    /// Move source to destination
    pub const fn mov(size: OperandSize, source: X86Register, destination: X86Register) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8 | OperandSize::S16);
        Self {
            size,
            opcode: 0x89,
            first_operand: source as u8,
            second_operand: destination as u8,
            ..Self::DEFAULT
        }
    }

    /// Move source to destination
    pub const fn mov_with_sign_extension(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8 | OperandSize::S16);
        Self {
            size,
            opcode: 0x63,
            first_operand: destination as u8,
            second_operand: source as u8,
            ..Self::DEFAULT
        }
    }

    /// Move to / from / between MMX (float mantissa)
    #[allow(dead_code)]
    pub const fn mov_mmx(size: OperandSize, source: X86Register, destination: X86Register) -> Self {
        exclude_operand_sizes!(
            size,
            OperandSize::S0 | OperandSize::S8 | OperandSize::S16 | OperandSize::S32
        );
        if (destination as u8) & 16 != 0 {
            // If the destination is a MMX register
            Self {
                size,
                opcode_escape_sequence: 1,
                opcode: if (source as u8) & 16 != 0 { 0x6F } else { 0x6E },
                first_operand: (destination as u8) & 0xF,
                second_operand: (source as u8) & 0xF,
                ..Self::DEFAULT
            }
        } else {
            Self {
                size,
                opcode_escape_sequence: 1,
                opcode: 0x7E,
                first_operand: (source as u8) & 0xF,
                second_operand: (destination as u8) & 0xF,
                ..Self::DEFAULT
            }
        }
    }

    /// Conditionally move source to destination
    pub const fn cmov(
        size: OperandSize,
        condition: u8,
        source: X86Register,
        destination: X86Register,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8 | OperandSize::S16);
        Self {
            size,
            opcode_escape_sequence: 1,
            opcode: condition,
            first_operand: destination as u8,
            second_operand: source as u8,
            ..Self::DEFAULT
        }
    }

    /// Swap source and destination
    pub const fn xchg(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(
            size,
            OperandSize::S0 | OperandSize::S8 | OperandSize::S16 | OperandSize::S32,
        );
        Self {
            size,
            opcode: 0x87,
            first_operand: source as u8,
            second_operand: destination as u8,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Swap byte order of destination
    pub const fn bswap(size: OperandSize, destination: X86Register) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8);
        match size {
            OperandSize::S16 => Self {
                size,
                opcode: 0xc1,
                second_operand: destination as u8,
                immediate_size: OperandSize::S8,
                immediate: 8,
                ..Self::DEFAULT
            },
            OperandSize::S32 | OperandSize::S64 => Self {
                size,
                opcode_escape_sequence: 1,
                opcode: 0xc8 | ((destination as u8) & 0b111),
                modrm: false,
                second_operand: destination as u8,
                ..Self::DEFAULT
            },
            _ => unimplemented!(),
        }
    }

    /// Test source and destination
    pub const fn test(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size,
            opcode: if let OperandSize::S8 = size {
                0x84
            } else {
                0x85
            },
            first_operand: source as u8,
            second_operand: destination as u8,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Test immediate and destination
    pub const fn test_immediate(
        size: OperandSize,
        destination: X86Register,
        immediate: i64,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size,
            opcode: if let OperandSize::S8 = size {
                0xf6
            } else {
                0xf7
            },
            first_operand: 0,
            second_operand: destination as u8,
            immediate_size: if let OperandSize::S64 = size {
                OperandSize::S32
            } else {
                size
            },
            immediate,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Compare source and destination
    pub const fn cmp(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size,
            opcode: if let OperandSize::S8 = size {
                0x38
            } else {
                0x39
            },
            first_operand: source as u8,
            second_operand: destination as u8,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Compare immediate and destination
    pub const fn cmp_immediate(
        size: OperandSize,
        destination: X86Register,
        immediate: i64,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size,
            opcode: if let OperandSize::S8 = size {
                0x80
            } else {
                0x81
            },
            first_operand: 7,
            second_operand: destination as u8,
            immediate_size: if let OperandSize::S64 = size {
                OperandSize::S32
            } else {
                size
            },
            immediate,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Load effective address of source into destination
    pub const fn lea(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
        indirect: Option<X86IndirectAccess>,
    ) -> Self {
        exclude_operand_sizes!(
            size,
            OperandSize::S0 | OperandSize::S8 | OperandSize::S16 | OperandSize::S32,
        );
        Self {
            size,
            opcode: 0x8d,
            first_operand: destination as u8,
            second_operand: source as u8,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Convert word to doubleword or doubleword to quadword
    pub const fn sign_extend_rax_rdx(size: OperandSize) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S8 | OperandSize::S16);
        Self {
            size,
            opcode: 0x99,
            modrm: false,
            ..X86Instruction::DEFAULT
        }
    }

    /// Load destination from [source + offset]
    pub const fn load(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
        indirect: X86IndirectAccess,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size: if let OperandSize::S64 = size {
                OperandSize::S64
            } else {
                OperandSize::S32
            },
            opcode_escape_sequence: match size {
                OperandSize::S8 | OperandSize::S16 => 1,
                _ => 0,
            },
            opcode: match size {
                OperandSize::S8 => 0xb6,
                OperandSize::S16 => 0xb7,
                _ => 0x8b,
            },
            first_operand: destination as u8,
            second_operand: source as u8,
            indirect: Some(indirect),
            ..Self::DEFAULT
        }
    }

    /// Store source in [destination + offset]
    pub const fn store(
        size: OperandSize,
        source: X86Register,
        destination: X86Register,
        indirect: X86IndirectAccess,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size,
            opcode: match size {
                OperandSize::S8 => 0x88,
                _ => 0x89,
            },
            first_operand: source as u8,
            second_operand: destination as u8,
            indirect: Some(indirect),
            ..Self::DEFAULT
        }
    }

    /// Load destination from immediate
    pub const fn load_immediate(destination: X86Register, immediate: i64) -> Self {
        let mut size = OperandSize::S64;
        if immediate >= 0 {
            if immediate <= u32::MAX as i64 {
                // Zero extend u32 imm to u64 reg
                size = OperandSize::S32;
            }
        } else if immediate >= i32::MIN as i64 {
            // Sign extend i32 imm to i64 reg
            return Self {
                size: OperandSize::S64,
                opcode: 0xc7,
                second_operand: destination as u8,
                immediate_size: OperandSize::S32,
                immediate,
                ..Self::DEFAULT
            };
        }
        // Load full u64 imm into u64 reg
        Self {
            size,
            opcode: (0xb8 | ((destination as u8) & 0b111)),
            modrm: false,
            second_operand: destination as u8,
            immediate_size: size,
            immediate,
            ..Self::DEFAULT
        }
    }

    /// Store sign-extended immediate in destination
    pub const fn store_immediate(
        size: OperandSize,
        destination: X86Register,
        indirect: X86IndirectAccess,
        immediate: i64,
    ) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0);
        Self {
            size,
            opcode: match size {
                OperandSize::S8 => 0xc6,
                _ => 0xc7,
            },
            second_operand: destination as u8,
            indirect: Some(indirect),
            immediate_size: if let OperandSize::S64 = size {
                OperandSize::S32
            } else {
                size
            },
            immediate,
            ..Self::DEFAULT
        }
    }

    /// Push source onto the stack
    pub const fn push_immediate(size: OperandSize, immediate: i32) -> Self {
        exclude_operand_sizes!(size, OperandSize::S0 | OperandSize::S16);
        Self {
            size,
            opcode: match size {
                OperandSize::S8 => 0x6A,
                _ => 0x68,
            },
            modrm: false,
            immediate_size: if let OperandSize::S64 = size {
                OperandSize::S32
            } else {
                size
            },
            immediate: immediate as i64,
            ..Self::DEFAULT
        }
    }

    /// Push source onto the stack
    pub const fn push(source: X86Register, indirect: Option<X86IndirectAccess>) -> Self {
        if indirect.is_none() {
            Self {
                size: OperandSize::S32,
                opcode: 0x50 | ((source as u8) & 0b111),
                modrm: false,
                second_operand: source as u8,
                ..Self::DEFAULT
            }
        } else {
            Self {
                size: OperandSize::S64,
                opcode: 0xFF,
                modrm: true,
                first_operand: 6,
                second_operand: source as u8,
                indirect,
                ..Self::DEFAULT
            }
        }
    }

    /// Pop from the stack into destination
    pub const fn pop(destination: X86Register) -> Self {
        Self {
            size: OperandSize::S32,
            opcode: 0x58 | ((destination as u8) & 0b111),
            modrm: false,
            second_operand: destination as u8,
            ..Self::DEFAULT
        }
    }

    /// Jump to relative destination on condition
    pub const fn conditional_jump_immediate(opcode: u8, relative_destination: i32) -> Self {
        Self {
            size: OperandSize::S32,
            opcode_escape_sequence: 1,
            opcode,
            modrm: false,
            immediate_size: OperandSize::S32,
            immediate: relative_destination as i64,
            ..Self::DEFAULT
        }
    }

    /// Jump to relative destination
    pub const fn jump_immediate(relative_destination: i32) -> Self {
        Self {
            size: OperandSize::S32,
            opcode: 0xe9,
            modrm: false,
            immediate_size: OperandSize::S32,
            immediate: relative_destination as i64,
            ..Self::DEFAULT
        }
    }

    /// Jump to absolute destination
    #[allow(dead_code)]
    pub const fn jump_reg(destination: X86Register, indirect: Option<X86IndirectAccess>) -> Self {
        Self {
            size: OperandSize::S64,
            opcode: 0xff,
            first_operand: 4,
            second_operand: destination as u8,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Push RIP and jump to relative destination
    pub const fn call_immediate(relative_destination: i32) -> Self {
        Self {
            size: OperandSize::S32,
            opcode: 0xe8,
            modrm: false,
            immediate_size: OperandSize::S32,
            immediate: relative_destination as i64,
            ..Self::DEFAULT
        }
    }

    /// Push RIP and jump to absolute destination
    pub const fn call_reg(destination: X86Register, indirect: Option<X86IndirectAccess>) -> Self {
        Self {
            size: OperandSize::S64,
            opcode: 0xff,
            first_operand: 2,
            second_operand: destination as u8,
            indirect,
            ..Self::DEFAULT
        }
    }

    /// Pop RIP
    pub const fn return_near() -> Self {
        Self {
            size: OperandSize::S32,
            opcode: 0xc3,
            modrm: false,
            ..Self::DEFAULT
        }
    }

    /// No operation
    #[allow(dead_code)]
    pub const fn noop() -> Self {
        Self {
            size: OperandSize::S32,
            opcode: 0x90,
            modrm: false,
            ..Self::DEFAULT
        }
    }

    /// Trap / software interrupt
    #[allow(dead_code)]
    pub const fn interrupt(immediate: u8) -> Self {
        if immediate == 3 {
            Self {
                size: OperandSize::S32,
                opcode: 0xcc,
                modrm: false,
                ..Self::DEFAULT
            }
        } else {
            Self {
                size: OperandSize::S32,
                opcode: 0xcd,
                modrm: false,
                immediate_size: OperandSize::S8,
                immediate: immediate as i64,
                ..Self::DEFAULT
            }
        }
    }

    /// rdtsc
    pub const fn cycle_count() -> Self {
        Self {
            size: OperandSize::S32,
            opcode_escape_sequence: 1,
            opcode: 0x31,
            modrm: false,
            ..Self::DEFAULT
        }
    }

    /// lfence / sfence / mfence
    #[allow(dead_code)]
    pub const fn fence(fence_type: FenceType) -> Self {
        Self {
            size: OperandSize::S32,
            opcode_escape_sequence: 1,
            opcode: 0xae,
            first_operand: fence_type as u8,
            ..Self::DEFAULT
        }
    }
}
