# five8

`five8` provides fast base58 encoding and decoding for 32-byte and 64-byte arrays.
It is a Rust port of [fd_base58](https://github.com/firedancer-io/firedancer/tree/main/src/ballet/base58).
There are four functions in the public api:

- `encode_32`
- `encode_64`
- `decode_32`
- `decode_64`

## Examples

### Encoding

```rust
let mut buf = [0u8; 44];
let bytes = &[
    24, 243, 6, 223, 230, 153, 210, 8, 92, 137, 123, 67, 164, 197, 79, 196, 125, 43, 183, 85,
    103, 91, 232, 167, 73, 131, 104, 131, 0, 101, 214, 231,
];
let len = five8::encode_32(bytes, &mut buf);
assert_eq!(
    &buf[..len as usize],
    [
        50, 103, 80, 105, 104, 85, 84, 106, 116, 51, 70, 74, 113, 102, 49, 86, 112, 105,
        100, 103, 114, 89, 53, 99, 90, 54, 80, 117, 121, 77, 99, 99, 71, 86, 119, 81, 72,
        82, 102, 106, 77, 80, 90, 71
    ]
);
assert_eq!(len, 44);
```

### Decoding

```rust
fn example_decode_32() {
    let bytes = b"2gPihUTjt3FJqf1VpidgrY5cZ6PuyMccGVwQHRfjMPZG";
    let mut out = [0u8; 32];
    five8::decode_32(bytes, &mut out).unwrap();
    assert_eq!(
        out,
        [
            24, 243, 6, 223, 230, 153, 210, 8, 92, 137, 123, 67, 164, 197, 79, 196, 125, 43,
            183, 85, 103, 91, 232, 167, 73, 131, 104, 131, 0, 101, 214, 231
        ]
    );
}
```

## Benchmarks

These benchmarks were run on a laptop with AVX2 support.
If your machine does not support AVX2 instructions it will be slower but should
still be faster than the alternatives - see the second set of benchmarks
where AVX2 is disabled.

### AVX2 enabled (`RUSTFLAGS='-C target-cpu-native`)

| Benchmark | five8   | [Lou-Kamades/fd_bs58][1] | [bs58-rs][2] |
| --------- | ------- | ------------------------ | ------------ |
| decode_32 | 36 ns   | 88 ns                    | 291 ns       |
| decode_64 | 124 ns  | 203 ns                   | 1092 ns      |
| encode_32 | 55 ns   | 96 ns                    | 682 ns       |
| encode_64 | 102 ns  | 209 ns                   | 2781 ns      |

[1]: https://github.com/Lou-Kamades/fd_bs58
[2]: https://github.com/Nullus157/bs58-rs

### AVX2 disabled (default `RUSTFLAGS`)

| Benchmark | five8   | [Lou-Kamades/fd_bs58][1] | [bs58-rs][2] |
| --------- | ------- | ------------------------ | ------------ |
| decode_32 | 49 ns   | 107 ns                   | 320 ns       |
| decode_64 | 162 ns  | 246 ns                   | 1176 ns      |
| encode_32 | 86 ns   | 98 ns                    | 824 ns       |
| encode_64 | 179 ns  | 219 ns                   | 3370 ns      |


### See Also

[`five8_const`](https://github.com/kevinheavey/five8/tree/main/crates): compile-time base58 decoding.