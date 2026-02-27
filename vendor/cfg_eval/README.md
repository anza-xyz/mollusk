# `::cfg_eval`

[`#[cfg_eval]`](
https://doc.rust-lang.org/1.70.0/core/prelude/v1/attr.cfg_eval.html
) in stable Rust.

[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](
https://github.com/danielhenrymantilla/cfg_eval.rs)
[![Latest version](https://img.shields.io/crates/v/cfg_eval.svg)](
https://crates.io/crates/cfg_eval)
[![Documentation](https://docs.rs/cfg_eval/badge.svg)](
https://docs.rs/cfg_eval)
[![MSRV](https://img.shields.io/badge/MSRV-stable--9-white)](https://gist.github.com/danielhenrymantilla/9b59de4db8e5f2467ed008b3c450527b)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](
https://github.com/rust-secure-code/safety-dance/)
[![License](https://img.shields.io/crates/l/cfg_eval.svg)](
https://github.com/danielhenrymantilla/cfg_eval.rs/blob/master/LICENSE-ZLIB)
[![CI](https://github.com/danielhenrymantilla/cfg_eval.rs/workflows/CI/badge.svg)](
https://github.com/danielhenrymantilla/cfg_eval.rs/actions)

<!-- Templated by `cargo-generate` using https://github.com/danielhenrymantilla/proc-macro-template -->

## Example

```rust
use ::macro_rules_attribute::apply;

#[macro_use]
extern crate cfg_eval;

fn main()
{
    let output_without_cfg_eval = {
        #[apply(stringify!)]
        enum Foo {
            Bar,

            #[cfg(FALSE)]
            NonExisting,
        }
    };
    // This is usually not great.
    assert!(output_without_cfg_eval.contains("NonExisting"));

    let output_with_cfg_eval = {
        #[cfg_eval]
        #[apply(stringify!)]
        enum Foo {
            Bar,

            #[cfg(FALSE)]
            NonExisting,
        }
    };
    assert_eq!(output_with_cfg_eval, stringify! {
        enum Foo {
            Bar,
        }
    });
}
```
