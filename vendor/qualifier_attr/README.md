# qualifier_attr

[![Latest Version](https://img.shields.io/crates/v/qualifier_attr.svg)][`qualifier_attr`]
[![Downloads](https://img.shields.io/crates/d/qualifier_attr.svg)][`qualifier_attr`]
[![Documentation](https://docs.rs/qualifier_attr/badge.svg)][`qualifier_attr`/docs]
[![License](https://img.shields.io/crates/l/qualifier_attr.svg)][`qualifier_attr`/license]
[![Dependency Status](https://deps.rs/repo/github/JohnScience/qualifier_attr/status.svg)][`qualifier_attr`/dep_status]

> Procedural macro attributes for adding "qualifiers" to various items.

At the moment, the crate supports the following "qualifiers":

* `pub`, `pub(crate)`, etc. - visibility and restriction
* `default` - default implementations (a feature of [specialization](https://doc.rust-lang.org/unstable-book/language-features/specialization.html))
* `async` - asynchronous code, e.g. `async fn`
* `unsafe` - unsafe code, e.g. `unsafe fn`, `unsafe trait`
* `const` - code that may run at compile time, e.g. `const fn`
* `extern "ABI"` - specifying an ABI, e.g. `extern "C" fn`

## Limitations

* It seems that rust-analyzer will sometimes complain when the attribute is
  used with modules.

## Examples

```rust
#[macro_use]
extern crate qualifier_attr;

// We can add a qualifier to a function
// with an attribute.
#[qualifiers(const)]
fn const_fn() -> u32 {
    42
}

const CONST_RES: u32 = const_fn();

// It's not so impressive on its own,
// but with `cfg_attr`, it can be conditional.
#[cfg_attr(feature = "extern_c", no_mangle, qualifiers(pub, extern "C"))]
fn extern_c_fn() -> u32 {
    42
}

// It even works with types, imports, and more!
mod foo {
    #[qualifiers(pub)]
    struct Foo {
        x: i32,
        y: i32,
    }
}

#[qualifiers(pub)]
use foo::Foo;

// Traits and implementations too!?
#[cfg_attr(feature = "unsafe_quux", qualifiers(unsafe))]
trait Quux {
    fn quux_the_thing();
}

#[cfg_attr(feature = "unsafe_quux", qualifiers(unsafe))]
impl Quux for Foo {
    fn quux_the_thing() {
        println!("The thing was quuxed.");
    }
}

// You can add qualifiers to the fields of a
// struct as well with this special attribute.
#[field_qualifiers(x(pub), y(pub))]
struct Point2 {
    x: i32,
    y: i32,
}

#[field_qualifiers(_0(pub), _1(pub), _2(pub))]
struct Point3(i32, i32, i32);
```

Learn more about `cfg_attr` [here](https://doc.rust-lang.org/reference/conditional-compilation.html#the-cfg_attr-attribute).

## Note on legacy attributes

Before version 0.2.0, this crate provided a separate attribute for each kind of item. While this should generally be a small win for compile times, it is generally not justified by the additional complexity. The legacy attributes are still available behind the default `legacy_attrs` feature flag, but their use is currently discouraged. In order to disable the legacy attributes, add the following to your `Cargo.toml`:

```toml
[dependencies]
qualifier_attr = { version = "0.2", default-features = false }
```

## Similar crates

* [`const_fn`](https://crates.io/crates/const_fn).

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

[`qualifier_attr`]: https://crates.io/crates/qualifier_attr
[`qualifier_attr`/docs]: https://docs.rs/qualifier_attr
[`qualifier_attr`/license]: https://github.com/JohnScience/qualifier_attr#license
[`qualifier_attr`/dep_status]: https://deps.rs/repo/github/JohnScience/qualifier_attr
