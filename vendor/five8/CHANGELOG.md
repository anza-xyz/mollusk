# Changelog

## [1.0.0] - 2025-07-12

Switch to `core::error`, remove std feature and set msrv to 1.81 [(#13)](https://github.com/kevinheavey/five8/pull/13)

## [0.2.1] - 2024-10-13

- Activate `std` feature of `five8_core` when `std` feature of `five8` is activated [(#7)](https://github.com/kevinheavey/five8/pull/7) 
- Add feature information to docs via `doc_auto_cfg` [(#7)](https://github.com/kevinheavey/five8/pull/7) 

## [0.2.0] - 2024-09-05

- Remove the `len` parameter from `encode_32` and `encode_64`, and just return the `len`.
- Re-export relevant items from `five8_const`

## [0.1.0] - 2024-07-30

First release!
