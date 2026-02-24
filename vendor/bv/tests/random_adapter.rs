// Random testing for bit vector adapters.

extern crate bv;

#[macro_use]
extern crate quickcheck;

use bv::{Bits, BitsExt, BitVec, BlockType};
use bv::adapter::BitSliceAdapter;

use quickcheck::{Arbitrary, Gen};

use std::cmp;

quickcheck! {
    fn prop_u8(program: Program) -> bool {
        program.evaluate::<u8>().check()
    }

    fn prop_u16(program: Program) -> bool {
        program.evaluate::<u16>().check()
    }

    fn prop_u32(program: Program) -> bool {
        program.evaluate::<u32>().check()
    }

    fn prop_u64(program: Program) -> bool {
        program.evaluate::<u64>().check()
    }

    fn prop_usize(program: Program) -> bool {
        program.evaluate::<usize>().check()
    }
}

// The result of evaluating a random program, on both the reference
// implementation and the actual implementation under test.
struct ProgramResult<Block: BlockType> {
    expected: RefImpl,
    actual:   Box<dyn Bits<Block = Block>>,
}

impl<Block: BlockType> ProgramResult<Block> {
    // Check that a program result is valid; that is, the vectors agree.
    fn check(&self) -> bool {
        self.expected == RefImpl::from_bits(&self.actual)
    }
}

#[derive(Clone, Debug)]
enum Program {
    Constant(RefImpl),
    Not(Box<Program>),
    And(Box<Program>, Box<Program>),
    Or(Box<Program>, Box<Program>),
    Xor(Box<Program>, Box<Program>),
    Concat(Box<Program>, Box<Program>),
    Slice(Box<Program>, usize, usize),
    Force(Box<Program>),
}

impl Arbitrary for Program {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        use Program::*;

        let recur  = |g: &mut G| Box::new(Program::arbitrary(g));
        let choice = g.gen_range(1, 71);

        match choice {
            01..=30 => Constant(RefImpl::arbitrary(g)),
            31..=35 => Not(recur(g)),
            36..=40 => And(recur(g), recur(g)),
            41..=45 => Or(recur(g), recur(g)),
            46..=50 => Xor(recur(g), recur(g)),
            51..=55 => Concat(recur(g), recur(g)),
            56..=60 => {
                let program = recur(g);
                let len     = program.len();
                if len >= 2 {
                    let start = g.gen_range(0, len / 2);
                    let limit = g.gen_range(len / 2, len);
                    Slice(program, start, limit - start)
                } else {
                    Slice(program, 0, len)
                }
            }
            _       => Force(Box::new(Program::arbitrary(g))),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item=Self>> {
        use Program::*;

        let add = |r: &mut Vec<Program>, p: &Box<Program>| r.push(Program::clone(&**p));

        let mut res = Vec::new();

        match *self {
            Constant(ref bv)       => res.extend(bv.shrink().map(Constant)),
            Not(ref p)             => add(&mut res, p),
            And(ref p1, ref p2)    => { add(&mut res, p1); add(&mut res, p2); }
            Or(ref p1, ref p2)     => { add(&mut res, p1); add(&mut res, p2); }
            Xor(ref p1, ref p2)    => { add(&mut res, p1); add(&mut res, p2); }
            Concat(ref p1, ref p2) => { add(&mut res, p1); add(&mut res, p2); }
            Slice(ref p, _, _)     => add(&mut res, p),
            Force(ref p)           => add(&mut res, p),
        }

        Box::new(res.into_iter())
    }
}

impl Program {
    fn len(&self) -> usize {
        use Program::*;

        match *self {
            Constant(ref bv)        => bv.0.len(),
            Not(ref p)              => p.len(),
            And(ref p1, ref p2)     => cmp::min(p1.len(), p2.len()),
            Or(ref p1, ref p2)      => cmp::min(p1.len(), p2.len()),
            Xor(ref p1, ref p2)     => cmp::min(p1.len(), p2.len()),
            Concat(ref p1, ref p2)  => p1.len() + p2.len(),
            Slice(_, _, len)        => len as usize,
            Force(ref p)            => p.len(),
        }
    }

    fn evaluate<Block: BlockType + 'static>(&self) -> ProgramResult<Block> {
        use Program::*;

        match *self {
            Constant(ref bv) => ProgramResult {
                expected: bv.clone(),
                actual:   Box::new(bv.to_bit_vec()),
            },

            Not(ref p) => {
                let res = p.evaluate();
                ProgramResult {
                    expected: res.expected.not(),
                    actual:   Box::new(res.actual.into_bit_not()),
                }
            }

            And(ref p1, ref p2) => {
                let res1 = p1.evaluate();
                let res2 = p2.evaluate();
                ProgramResult {
                    expected: res1.expected.and(&res2.expected),
                    actual:   Box::new(res1.actual.into_bit_and(res2.actual)),
                }
            }

            Or(ref p1, ref p2) => {
                let res1 = p1.evaluate();
                let res2 = p2.evaluate();
                ProgramResult {
                    expected: res1.expected.or(&res2.expected),
                    actual:   Box::new(res1.actual.into_bit_or(res2.actual)),
                }
            }

            Xor(ref p1, ref p2) => {
                let res1 = p1.evaluate();
                let res2 = p2.evaluate();
                ProgramResult {
                    expected: res1.expected.xor(&res2.expected),
                    actual:   Box::new(res1.actual.into_bit_xor(res2.actual)),
                }
            }

            Concat(ref p1, ref p2) => {
                let res1 = p1.evaluate();
                let res2 = p2.evaluate();
                ProgramResult {
                    expected: res1.expected.concat(&res2.expected),
                    actual:   Box::new(res1.actual.into_bit_concat(res2.actual)),
                }
            }

            Slice(ref p, start, len) => {
                let res = p.evaluate();
                ProgramResult {
                    expected: res.expected.slice(start, len),
                    actual:   Box::new(BitSliceAdapter::new(res.actual, start as u64, len as u64)),
                }
            }

            Force(ref p) => {
                let res = p.evaluate();
                ProgramResult {
                    expected: res.expected,
                    actual:   Box::new(res.actual.to_bit_vec()),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RefImpl(Vec<bool>);

impl Arbitrary for RefImpl {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        RefImpl(<Vec<bool>>::arbitrary(g))
    }

    fn shrink(&self) -> Box<dyn Iterator<Item=Self>> {
        Box::new(self.0.shrink().map(RefImpl))
    }
}

impl RefImpl {
    fn from_bits<T: Bits>(bits: &T) -> Self {
        let mut result = RefImpl(Vec::with_capacity(bits.bit_len() as usize));
        for i in 0 .. bits.bit_len() {
            result.0.push(bits.get_bit(i));
        }
        result
    }

    fn to_bit_vec<Block: BlockType>(&self) -> BitVec<Block> {
        let mut result = BitVec::with_capacity(self.0.len() as u64);
        for i in 0 .. self.0.len() {
            result.push(self.0[i])
        }
        result
    }

    fn not(&self) -> Self {
        RefImpl(self.0.iter().map(|&b| !b).collect())
    }

    fn bin_op<F: Fn(bool, bool) -> bool>(&self, other: &RefImpl, op: F) -> Self {
        RefImpl(self.0.iter().zip(other.0.iter())
            .map(|(&b1, &b2)| op(b1, b2)).collect())
    }

    fn and(&self, other: &RefImpl) -> Self {
        self.bin_op(other, |b1, b2| b1 & b2)
    }

    fn or(&self, other: &RefImpl) -> Self {
        self.bin_op(other, |b1, b2| b1 | b2)
    }

    fn xor(&self, other: &RefImpl) -> Self {
        self.bin_op(other, |b1, b2| b1 ^ b2)
    }

    fn concat(&self, other: &RefImpl) -> Self {
        let mut result = self.clone();
        result.0.extend(other.0.iter().cloned());
        result
    }

    fn slice(&self, start: usize, len: usize) -> Self {
        let mut result = RefImpl(Vec::new());
        for i in start .. (start + len) {
            result.0.push(self.0[i]);
        }
        result
    }
}

