# bv-rs: bit-vectors and bit-slices for Rust

[![Build Status](https://travis-ci.org/tov/bv-rs.svg?branch=master)](https://travis-ci.org/tov/bv-rs)
[![Crates.io](https://img.shields.io/crates/v/bv.svg?maxAge=2592000)](https://crates.io/crates/bv)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE-APACHE)

The main type exported by the library, `BitVec`, is a packed, growable
bit-vector. Its API mirrors that of `Vec` where reasonable. The library
also defines slice operations that return `BitSlice` or `BitSliceMut`,
akin to Rust’s array slices but for bit-vectors. A common API to
bit-vectors and bit-slices is provided by the `Bits`, `BitsMut`, and
`BitsPush` traits, which also allow treating as bit-vectors all primitive 
unsigned integer types (`uN`), and vectors and slices thereof, as well
as unpacked vectors and slices of `bool`.

## Usage

It’s [on crates.io](https://crates.io/crates/bv), so you can add

```toml
[dependencies]
bv = "0.11.1"
```

to your `Cargo.toml` and

```rust
extern crate bv;
```

to your crate root.

This crate supports Rust version 1.31 and newer.
