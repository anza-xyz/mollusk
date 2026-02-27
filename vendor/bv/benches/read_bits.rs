#![feature(test)]

extern crate bv;
extern crate test;

use bv::{Bits, BitsMut, BitVec, BitSliceable};
use test::Bencher;

const NBITS: u64 = 640;

#[inline(never)]
fn mystery_bit() -> bool { false }

#[bench]
fn bit_vec_read_bits(b: &mut Bencher) {
    let bv: BitVec = BitVec::new_fill(true, NBITS);

    b.iter(|| {
        let mut result = false;

        for i in 0 .. bv.len() {
            result ^= bv[i];
        }

        result
    });
}

#[bench]
fn bit_vec_read_bits_write(b: &mut Bencher) {
    let mut bv: BitVec = BitVec::new_fill(true, NBITS);

    b.iter(|| {
        for i in 0 .. bv.len() {
            let bit = bv[i] ^ mystery_bit();
            bv.set_bit(i, bit);
        }
    });
}

#[bench]
fn slice_read_bits(b: &mut Bencher) {
    let v = vec![0u64; NBITS as usize / 64];
    let slice = v.as_slice();

    b.iter(|| {
        let mut result = false;

        for i in 0 .. slice.bit_len() {
            result ^= slice.get_bit(i as u64);
        }

        result
    });
}

#[bench]
fn slice_read_bits_write(b: &mut Bencher) {
    let mut v = vec![0u64; NBITS as usize / 64];
    let slice = v.as_mut_slice();

    b.iter(|| {
        for i in 0 .. slice.bit_len() {
            let bit = slice.get_bit(i) ^ mystery_bit();
            slice.set_bit(i, bit);
        }
    });
}

#[bench]
fn bit_slice_read_bits(b: &mut Bencher) {
    let v = vec![0u64; NBITS as usize / 64];
    let slice = v.bit_slice(..);

    b.iter(|| {
        let mut result = false;

        for i in 0 .. slice.bit_len() {
            result ^= slice.get_bit(i as u64);
        }

        result
    });
}

#[bench]
fn bit_slice_read_bits_write(b: &mut Bencher) {
    let mut v = vec![0u64; NBITS as usize / 64];
    let mut slice = (&mut *v).bit_slice(..);

    b.iter(|| {
        for i in 0 .. slice.bit_len() {
            let bit = slice.get_bit(i) ^ mystery_bit();
            slice.set_bit(i, bit);
        }
    });
}

