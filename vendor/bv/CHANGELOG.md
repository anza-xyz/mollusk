# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog] and this project adheres to
[Semantic Versioning].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: http://semver.org/spec/v2.0.0.html

<!-- next-header -->

## [Unreleased]

## [0.11.1] - 2020-03-18

### Fixed
- Checks invariants after deserialization. (Thanks, @Mrmaxmeier!)

### Changed
- Oldest supported rustc version is now 1.31.0.

## [0.11.0] - 2019-01-16

### Fixed
- `BitVec::get` takes `&self`, not `&mut self`.

### Changed
- Oldest supported rustc version is now 1.24.0.

## [0.10.0] - 2018-06-19

### Added
- `BitsMutExt` extension trait with methods `bit_assign`, 
  `bit_zip_assign`, `bit_and_assign`, `bit_or_assign`, and `bit_xor_assign`.
- `BitSliceableMut` helper trait with one method, `bit_slice_mut`. This
  trait is automatically implemented for all `BitSliceable` types that slice to
  a `BitsMut` type. Calling `bit_slice_mut` forces auto-ref to choose such a 
  type.
- `Bits::get_raw_block` method, which may return spurious bits in the
  last block of a bit vector. This enables some fast paths where we don't
  mind the spurious bits.
- `From<Box<[Block]>>` and `From<Vec<Block>>` impls for `BitVec<Block>`.

### Changed
- `Bits` is now a super trait of `BitSliceable<R>`, and the block type of
  the slice matches the block type of `Self`.
- The function passed to `BitsExt::bit_zip` no longer takes a third, `usize` 
  argument.
  
### Removed
- `BlockType::last_block_bits` in favor of `BlockType::block_bits`.
- Alias of trait `BitsExt` is moved in the `adapter` module. (Deprecated in
  0.8.1.)
  
## [0.9.0] - 2018-06-18

### Added
- Impls of `Bits` and `BitsMut` for `Vec<Block>`. The impls for `[Block]`
  already caught most causes because of autoderef, but this is helpful
  when you want to pass a `Vec` to a generic method.
  
### Changed
- `BitSlice::from_raw_parts` and `BitSliceMut::from_raw_parts` now take
  an offset of type `u64` rather than `u8`, and the offset doesn't have to
  be less than the block size.
- Removed 12 `BitSliceable` impls for array slices with two blanket impls.
  I don't believe this is a breaking change.
  
### Improved
- Single-bit `BitVec` operations are significantly faster now.

### Fixed
- Bit-slicing `&[bool]` and `&mut [bool]` using `BitSliceable` now requires
  `u64` ranges rather than `usize` ranges. Now all `BitSliceable` instances
  use `u64` ranges.

## [0.8.2] - 2018-06-15

### Fixed
- Empty `BitVec` does not allocate.

## [0.8.1] - 2018-06-14

### Added
- `BitsExt::bit_zip` method generalizes `bit_and`, `bit_or`, etc.
- `BlockType::block_bits` static method, for finding out the number
  of bits in the block at the given position.

### Changed
- Trait `BitsExt` is moved from the `adapter` module to the root. (There's
  still a `pub use` for it in `adapter`, but that alias is deprecated and will
  be removed in 0.10.0.)
  
### Fixed
- Performance problem with `BitVec::block_reserve`, which was growing way
  too much.

## [0.8.0] - 2018-06-13

### Added
- `adapter` module, including:
  - `BitsExt` trait, for adapter operations over types that 
    implement `Bits`. Methods include:
      - bit-wise logic: `bit_not`, `bit_and`, `bit_or`, and `bit_xor`;
      - `bit_concat` and `bit_pad`.
  - constant adapter `BitFill`;
  - `BoolAdapter` for adapting unpacked `Vec<bool>` or `&[bool]`; and
  - slicing adapter `BitSliceAdapter`. (Note that `BitSliceAdapter` does 
    not replace the more specialized `BitSlice`.)
- `Bits::to_bit_vec` method, which copies the bits of a `Bits` into a new 
  `BitVec`.
- `bit_vec!` macro allows trailing comma.
- `BitVec::get` and `BitVec::set` methods, aliasing `Bits::get_bit` and
  `BitsMut::set_bit`, respectively.
- `From` impls for converting array slices to `BitSlice`, mutable array 
  slices to `BitSliceMut`, and `BitSliceMut` to `BitSlice`.
- `Bits`, `BitsMut`, and `BitSliceable` impls for sized arrays from sizes 0
  to 31.
  
### Changed
- `BitSliceMut::as_immut` renamed to `BitSliceMut::as_bit_slice`.
  
### Removed
- `Bits::bit_offset` method. Blocks returned from a `Bits` instance are now
  assumed to be aligned. This makes `BitSlice` more complicated but 
  significantly simplifies everything else and removes some infelicities.
- `BitSliceBlockIter` struct and methods for creating it. I'm rethinking how
  iteration over bit vectors and bit-vector-likes should work.

## [0.7.2] - 2018-05-16

### Added
- A good number of doc examples.
- `html_doc_root` for cross-linking of docs.

## [0.7.1] - 2018-05-13

### Fixed
- Documentation, in many places.

## [0.7.0] - 2018-05-13

### Changed
- Renamed about half the types in the library. Doing this while it
  has no users! Changes are:
   - structures `BitVec{,Mut,Push}` -> `Bits{,Mut,Push}`
   - structure `Bv` -> `BitVec`
   - module `bv` -> `bit_vec`
   - macro `bv!` -> `bit_vec!`

## [0.6.6] - 2018-05-12

### Added
- Support for inclusive ranges.

## [0.6.4] - 2018-05-12

### Added
- Optional `serde` support, via Cargo feature `"serde"`.

### Removed
- Cargo feature `"u128"`, as `u128` support is now detected.

## [0.6.0] - 2018-05-11

### Changed
- Support for `u128` is now detected.
- No longer depends on `num-traits` crate.

## [0.5.0] - 2018-05-11

### Added
- Support for `u128`, enabled by Cargo feature `"u128"`.
- Version support statement: rustc 1.20.0.

## [0.4.4] - 2018-05-10

### Fixed
- Documentation URL.
- Version number in example toml.

## [0.4.3] - 2018-03-31

### Added
- Impl of `Default` for `BV`.
- Methods `BitSlice::is_empty` and `BitSliceMut::is_empty`.

## [0.4.2] - 2018-03-30

### Added
- Lots of tests.

### Fixed
- Added a necessary lifetime parameter.

## [0.4.1] - 2018-03-30

### Fixed
- `BV::align_block` and `BV::resize`.

## [0.4.0] - 2018-03-30

### Added
- Impl `BitSliceable` for `[bool]` and `Vec<bool>`.

### Changed
- Renamed method `BitSliceable::slice` to `BitSliceable::bit_slice`.

## [0.3.1] - 2018-03-30

### Added
- More tests.

### Fixed
- Bug in `block_len` method.

## [0.3.0] - 2018-03-28

### Added
- `bv!` macro.
- `BitSlice::len` and `BitSliceMut::len` methods.
- `BV::resize` method.

## [0.2.0] - 2018-03-28

### Added
- impl `BitVec` and `BitVecMut` for `[bool]`.

### Fixed
- Documentation.

## [0.1.2] - 2018-03-28

### Added
- Better crate summary in README.
- Documentation on bit slice representation.

## [0.1.1] - 2018-03-28

### Fixed
- Crate metadata.

## [0.1.0] - 2018-03-28

Initial release.
