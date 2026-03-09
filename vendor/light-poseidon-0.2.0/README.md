[![Crates.io](https://img.shields.io/crates/v/light-poseidon.svg)](https://crates.io/crates/light-poseidon)
[![Workflow Status](https://github.com/Lightprotocol/light-poseidon/workflows/main/badge.svg)](https://github.com/Lightprotocol/light-poseidon/actions?query=workflow)

# light-poseidon

<!-- cargo-rdme start -->

**light-poseidon** is a [Poseidon](https://eprint.iacr.org/2019/458) hash
implementation in Rust created for [Light Protocol](https://www.lightprotocol.com/).

## Parameters

The library provides pre-generated parameters over the BN254 curve, however
it can work with any parameters provided as long as developers take care
of generating the round constants.

Parameters provided by the library are:

* *x^5* S-boxes
* width - *2 ≤ t ≤ 13*
* inputs - *1 ≤ n ≤ 12*
* 8 full rounds and partial rounds depending on *t*: *[56, 57, 56, 60, 60, 63, 64, 63, 60, 66, 60, 65]*

The parameters can be generated with:

```bash
cargo xtask generate-poseidon-parameters
````

## Output type

[`Poseidon`](https://docs.rs/light-poseidon/latest/light_poseidon/struct.Poseidon.html) type implements two traits which serve the purpose
of returning the calculated hash in different representations:

* [`PoseidonBytesHasher`](https://docs.rs/light-poseidon/latest/light_poseidon/trait.PoseidonBytesHasher.html) with the
  `hash_bytes_be` and `hash_bytes_le` methods which returns a byte array.
* [`PoseidonHasher`](https://docs.rs/light-poseidon/latest/light_poseidon/trait.PoseidonHasher.html) with the `hash` method which returns
  [`ark_ff::PrimeField`](ark_ff::PrimeField). Might be useful if you want
  to immediately process the result with an another library which works with
  [`ark_ff::PrimeField`](ark_ff::PrimeField) types.

## Examples

Example with two simple big-endian byte inputs (converted to field elements)
and BN254-based parameters provided by the library, with
[`PoseidonBytesHasher`](https://docs.rs/light-poseidon/latest/light_poseidon/trait.PoseidonHasher.html) trait and a byte array
result:

```rust
use light_poseidon::{Poseidon, PoseidonBytesHasher, parameters::bn254_x5};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();

let hash = poseidon.hash_bytes_be(&[&[1u8; 32], &[2u8; 32]]).unwrap();

println!("{:?}", hash);
// Should print:
// [
//     13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123, 132,
//     254, 156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144
// ]
```

With [`PoseidonHasher`](https://docs.rs/light-poseidon/latest/light_poseidon/trait.PoseidonHasher.html) trait and
[`ark_ff::PrimeField`](ark_ff::PrimeField) result:

```rust
use light_poseidon::{Poseidon, PoseidonHasher, parameters::bn254_x5};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();

let input1 = Fr::from_be_bytes_mod_order(&[1u8; 32]);
let input2 = Fr::from_be_bytes_mod_order(&[2u8; 32]);

let hash = poseidon.hash(&[input1, input2]).unwrap();

// Do something with `hash`.
```

## Implementation

The implementation is compatible with the
[original SageMath implementation](https://extgit.iaik.tugraz.at/krypto/hadeshash/-/tree/master/),
but it was also inspired by the following ones:

* [circomlibjs](https://github.com/iden3/circomlibjs)
* [zero-knowledge-gadgets](https://github.com/webb-tools/zero-knowledge-gadgets)

## Performance

This repository contains a benchmark measuring the performance of this
Poseidon implementation for given 1 - 12 random 32 bytes inputs.

To run them, simply use:

```bash
cargo bench
```

This is the result from a host with the following hardware:

* 12th Gen Intel® Core™ i7-1260P × 16

```norust
poseidon_bn254_x5_1     time:   [17.543 µs 18.303 µs 19.133 µs]
Found 9 outliers among 100 measurements (9.00%)
  9 (9.00%) high mild

poseidon_bn254_x5_2     time:   [25.020 µs 25.866 µs 26.830 µs]

poseidon_bn254_x5_3     time:   [36.076 µs 37.549 µs 38.894 µs]

poseidon_bn254_x5_4     time:   [50.333 µs 52.598 µs 54.806 µs]

poseidon_bn254_x5_5     time:   [64.184 µs 66.324 µs 68.706 µs]

poseidon_bn254_x5_6     time:   [87.356 µs 90.259 µs 93.437 µs]

poseidon_bn254_x5_7     time:   [120.08 µs 125.26 µs 130.23 µs]

poseidon_bn254_x5_8     time:   [134.28 µs 139.65 µs 145.71 µs]

poseidon_bn254_x5_9     time:   [161.99 µs 168.93 µs 175.94 µs]

poseidon_bn254_x5_10    time:   [208.11 µs 215.27 µs 222.99 µs]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

poseidon_bn254_x5_11    time:   [239.47 µs 249.05 µs 258.35 µs]

poseidon_bn254_x5_12    time:   [295.47 µs 305.80 µs 316.41 µs]
```

## Security

This library has been audited by [Veridise](https://veridise.com/). You can
read the audit report [here](https://github.com/Lightprotocol/light-poseidon/blob/main/assets/audit.pdf).

<!-- cargo-rdme end -->
