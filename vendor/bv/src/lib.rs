#![doc(html_root_url = "https://tov.github.io/bv-rs")]
//! The main type exported by the library, [`BitVec`], is a packed,
//! growable bit-vector. Its API mirrors that of `Vec` where reasonable.
//!
//! The library also defines slice operations that return
//! [`BitSlice`] or [`BitSliceMut`], akin to Rust’s array slices but for
//! bit-vectors. A common API to bit-vectors and bit-slices is provided by the [`Bits`],
//! [`BitsMut`], and [`BitsPush`] traits. These traits also allow treating a variety
//! of other types as bit vectors:
//!
//!  - all primitive unsigned integer types (*e.g.,* `u64`, `u32`),
//!  - vectors and slices thereof (*e.g.*, `Vec<usize>`, `&[u8]`, `[u16; 4]`), and
//!  - unpacked vectors and arrays of `bool` (*e.g.*, `[bool; 15]`).
//!
//! Additionally, the [`BitsExt`] trait provides adapter methods including
//! bit-wise logic and concatenation. These adapters work for all types that implement
//! [`Bits`].
//!
//! # Examples
//!
//! A first example with [`BitVec`]:
//!
//! ```
//! use bv::BitVec;
//!
//! let mut bv1: BitVec = BitVec::new_fill(false, 50);
//! let mut bv2: BitVec = BitVec::new_fill(false, 50);
//!
//! assert_eq!(bv1, bv2);
//!
//! bv1.set(49, true);
//! assert_ne!(bv1, bv2);
//!
//! assert_eq!(bv1.pop(), Some(true));
//! assert_eq!(bv2.pop(), Some(false));
//! assert_eq!(bv1, bv2);
//! ```
//!
//! Adapters, from [`BitsExt`] and [`adapter`]:
//!
//! ```
//! use bv::*;
//! use bv::adapter::BoolAdapter;
//!
//! // Here, we use an `&[u16]` as a bit vector, and we adapt a
//! // `Vec<bool>` as well.
//! let array = &[0b1100u16];
//! let vec   = BoolAdapter::new(vec![false, true, false, true]);
//!
//! // `xor` is not a `BitVec`, but a lazy adapter, thus, we can index
//! // it or efficiently compare it to another bit vector, without
//! // allocating.
//! let xor   = array.bit_xor(&vec);
//! assert_eq!( xor, bit_vec![false, true, true, false] );
//! ```
//!
//! This function performs a three-way *or*, returning a `BitVec` without
//! allocating an intermediate result:
//!
//! ```
//! use bv::{Bits, BitsExt, BitVec};
//!
//! fn three_way_or<T, U, V>(bv1: T, bv2: U, bv3: V) -> BitVec<T::Block>
//!     where T: Bits,
//!           U: Bits<Block = T::Block>,
//!           V: Bits<Block = T::Block> {
//!
//!     bv1.into_bit_or(bv2).into_bit_or(bv3).to_bit_vec()
//! }
//! ```
//!
//! # Usage
//!
//! It’s [on crates.io](https://crates.io/crates/bv), so you can add
//!
//! ```toml
//! [dependencies]
//! bv = "0.11.1"
//! ```
//!
//! to your `Cargo.toml` and
//!
//! ```rust
//! extern crate bv;
//! ```
//!
//! to your crate root.
//!
//! This crate supports Rust version 1.31 and newer.
//!
//! [`BitVec`]: struct.BitVec.html
//! [`Bits`]: trait.Bits.html
//! [`BitsMut`]: trait.BitsMut.html
//! [`BitsPush`]: trait.BitsPush.html
//! [`BitSlice`]: struct.BitSlice.html
//! [`BitSliceMut`]: struct.BitSliceMut.html
//! [`BitsExt`]: trait.BitsExt.html
//! [`adapter`]: adapter/index.html

#![warn(missing_docs)]

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod range_compat;

#[macro_use]
mod macros;

mod storage;
pub use self::storage::BlockType;

mod traits;
pub use self::traits::{Bits, BitsExt, BitsMut, BitsMutExt, BitsPush,
                       BitSliceable, BitSliceableMut};

mod slice;
pub use self::slice::{BitSlice, BitSliceMut};

mod bit_vec;
pub use self::bit_vec::BitVec;

mod array_n_impls;
mod iter;
mod prims;

pub mod adapter;
