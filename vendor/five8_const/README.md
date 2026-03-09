# five8_const

This crate provides compile-time base58 decoding.

It exposes four functions:
- `try_decode_32_const`
- `try_decode_64_const`
- `decode_32_const`
- `decode_64_const`

While the first two functions return `Result` types,
the latter two are more useful for declaring constants:

```rust
const EXAMPLE: [u8; 32] = five8_const::decode_32_const("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFF");
```

If you want to do base58 encoding or decoding at runtime,
just use the [five8](https://github.com/kevinheavey/five8/tree/main/crates/five8)
crate. It's faster.
