Educe
====================

[![CI](https://github.com/magiclen/educe/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/educe/actions/workflows/ci.yml)

This crate offers procedural macros designed to facilitate the swift implementation of Rust's built-in traits.

## Features

By default, every trait this crate supports will be enabled. You can disable all of them by turning off the default features and enable only the traits that you want to use by adding them to the `features` explicitly.

For example,

```toml
[dependencies.educe]
version = "*"
features = ["Debug", "Clone", "Copy", "Hash", "Default"]
default-features = false
```

## Traits

#### Debug

Use `#[derive(Educe)]` and `#[educe(Debug)]` to implement the `Debug` trait for a struct, enum, or union. This allows you to modify the names of your types, variants, and fields. You can also choose to ignore specific fields or set a method to replace the `Debug` trait. Additionally, you have the option to format a struct as a tuple and vice versa.

###### Basic Usage

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Debug)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Change the Name of a Type, a Variant or a Field

The `name` parameter can rename a type, a variant or a field. If you set it to `false`, the name can be ignored or forced to show otherwise.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug(name(Struct2)))]
struct Struct {
    #[educe(Debug(name(f)))]
    f1: u8
}

#[derive(Educe)]
#[educe(Debug(name = true))]
enum Enum {
    #[educe(Debug(name = false))]
    V1,
    #[educe(Debug(name(V)))]
    V2 {
        #[educe(Debug(name(f)))]
        f1: u8,
    },
    #[educe(Debug(name = false))]
    V3(u8),
}
```

###### Ignore Fields

The `ignore` parameter can ignore a specific field.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug)]
struct Struct {
    #[educe(Debug(ignore))]
    f1: u8
}

#[derive(Educe)]
#[educe(Debug)]
enum Enum {
    V1,
    V2 {
        #[educe(Debug(ignore))]
        f1: u8,
    },
    V3(
        #[educe(Debug(ignore))]
        u8
    ),
}
```

###### Fake Structs and Tuples

With the `named_field` parameter, structs can be formatted as tuples and tuples can be formatted as structs.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug(named_field = false))]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Debug)]
enum Enum {
    V1,
    #[educe(Debug(named_field = false))]
    V2 {
        f1: u8,
    },
    #[educe(Debug(named_field = true))]
    V3(
        u8,
        #[educe(Debug(name(value)))]
        i32
    ),
}
```

###### Use Another Method to Handle the Formatting

The `method` parameter can be utilized to replace the implementation of the `Debug` trait for a field, eliminating the need to implement the `Debug` trait for the type of that field.

```rust
use educe::Educe;

use std::fmt::{self, Formatter};

fn fmt<T>(_s: &T, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str("Hi")
}

#[derive(Educe)]
#[educe(Debug)]
enum Enum<T> {
    V1,
    V2 {
        #[educe(Debug(method(fmt)))]
        f1: u8,
    },
    V3(
        #[educe(Debug(method(std::fmt::UpperHex::fmt)))]
        u8,
        #[educe(Debug(method(fmt)))]
        T
    ),
}
```

###### Generic Parameters Bound to the `Debug` Trait or Others

Generic parameters will be automatically bound to the `Debug` trait if necessary.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

use std::fmt::{self, Formatter};

fn fmt<D>(_s: &D, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str("Hi")
}

#[derive(Educe)]
#[educe(Debug(bound(T: std::fmt::Debug)))]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(Debug(method(fmt)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

In the above case, `T` is bound to the `Debug` trait, but `K` is not.

Or, you can have `educe` replicate the behaviour of `std`'s `derive`'s, where a bound is produced for *every* generic parameter, without regard to how it's used in the structure:

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug(bound(*)))]
struct Struct<T> {
    #[educe(Debug(ignore))]
    f: T,
}
```

This can be useful if you don't want to make the trait implementation part of your permanent public API. In this example, `Struct<T>` doesn't implement `Debug` unless `T` does. I.e., it has a `T: Debug` bound even though that's not needed right now. Later we might want to display `f`; we wouldn't then need to make a breaking API change by adding the bound.

This was the behaviour of `Trait(bound)` in educe 0.4.x and earlier.

###### Union

A union will be formatted as a `u8` slice because we don't know its fields at runtime. The fields of a union cannot be ignored, renamed, or formatted with other methods. The implementation is **unsafe** because it may expose uninitialized memory.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Debug(unsafe))]
union Union {
    f1: u8,
    f2: i32,
}
```

#### Clone

Use `#[derive(Educe)]` and `#[educe(Clone)]` to implement the `Clone` trait for a struct, an enum, or a union. You can set a method to replace the `Clone` trait.

###### Basic Usage

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Clone)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Clone)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Use Another Method to Perform Cloning

The `method` parameter can be utilized to replace the implementation of the `Clone` trait for a field, eliminating the need to implement the `Clone` trait for the type of that field.

```rust
use educe::Educe;

fn clone(v: &u8) -> u8 {
    v + 100
}

trait A {
    fn add(&self, rhs: u8) -> Self;
}

fn clone2<T: A>(v: &T) -> T {
    v.add(100)
}

#[derive(Educe)]
#[educe(Clone)]
enum Enum<T: A> {
    V1,
    V2 {
        #[educe(Clone(method(clone)))]
        f1: u8,
    },
    V3(
        #[educe(Clone(method(clone2)))]
        T
    ),
}
```

###### Generic Parameters Bound to the `Clone` Trait or Others

Generic parameters will be automatically bound to the `Clone` trait if necessary. If the `#[educe(Copy)]` attribute exists, they will be bound to the `Copy` trait.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Clone)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

trait A {
    fn add(&self, rhs: u8) -> Self;
}

fn clone<T: A>(v: &T) -> T {
    v.add(100)
}

#[derive(Educe)]
#[educe(Clone(bound(T: std::clone::Clone)))]
enum Enum<T, K: A> {
    V1,
    V2 {
        #[educe(Clone(method(clone)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

In the above case, `T` is bound to the `Clone` trait, but `K` is not.

Or, you can have `educe` replicate the behaviour of `std`'s `derive`'s by using `bound(*)`. See the [`Debug`](#debug) section for more information.

```rust
use educe::Educe;

trait A {
    fn add(&self, rhs: u8) -> Self;
}

fn clone<T: A>(v: &T) -> T {
    v.add(100)
}

#[derive(Educe)]
#[educe(Clone(bound(*)))]
struct Struct<T: A> {
    #[educe(Clone(method(clone)))]
    f: T,
}
```

###### Union

Refer to the introduction of the `#[educe(Copy)]` attribute.

#### Copy

Use `#[derive(Educe)]` and `#[educe(Copy)]` to implement the `Copy` trait for a struct, an enum, or a union.

###### Basic Usage

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Copy, Clone)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Copy, Clone)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Generic Parameters Bound to the `Copy` Trait or Others

All generic parameters will be automatically bound to the `Copy` trait.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Copy, Clone)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

trait A {
    fn add(&self, rhs: u8) -> Self;
}

fn clone<T: A>(v: &T) -> T {
    v.add(100)
}

#[derive(Educe)]
#[educe(Copy, Clone(bound(T: Copy, K: A + Copy)))]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(Clone(method(clone)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

Note that utilizing custom cloning methods for a type that implements the `Copy` and `Clone` traits may not be entirely appropriate.

###### Union

The `#[educe(Copy, Clone)]` attribute can be used for a union. The fields of a union cannot be cloned with other methods.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Copy, Clone)]
union Union {
    f1: u8,
}
```

#### PartialEq

Use `#[derive(Educe)]` and `#[educe(PartialEq)]` to implement the `PartialEq` trait for a struct, enum, or union. You can also choose to ignore specific fields or set a method to replace the `PartialEq` trait.

###### Basic Usage

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(PartialEq)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Ignore Fields

The `ignore` parameter can ignore a specific field.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq)]
struct Struct {
    #[educe(PartialEq(ignore))]
    f1: u8
}

#[derive(Educe)]
#[educe(PartialEq)]
enum Enum {
    V1,
    V2 {
        #[educe(PartialEq(ignore))]
        f1: u8,
    },
    V3(
        #[educe(PartialEq(ignore))]
        u8
    ),
}
```

###### Use Another Method to Perform Comparison

The `method` parameter can be utilized to replace the implementation of the `PartialEq` trait for a field, eliminating the need to implement the `PartialEq` trait for the type of that field.

```rust
use educe::Educe;

fn eq(a: &u8, b: &u8) -> bool {
    a + 1 == *b
}

trait A {
    fn is_same(&self, other: &Self) -> bool;
}

fn eq2<T: A>(a: &T, b: &T) -> bool {
    a.is_same(b)
}

#[derive(Educe)]
#[educe(PartialEq)]
enum Enum<T: A> {
    V1,
    V2 {
        #[educe(PartialEq(method(eq)))]
        f1: u8,
    },
    V3(
        #[educe(PartialEq(method(eq2)))]
        T
    ),
}
```

###### Generic Parameters Bound to the `PartialEq` Trait or Others

Generic parameters will be automatically bound to the `PartialEq` trait if necessary.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

trait A {
    fn is_same(&self, other: &Self) -> bool;
}

fn eq<T: A>(a: &T, b: &T) -> bool {
    a.is_same(b)
}

#[derive(Educe)]
#[educe(PartialEq(bound(T: std::cmp::PartialEq, K: A)))]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(PartialEq(method(eq)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

In the above case, `T` is bound to the `PartialEq` trait, but `K` is not.

You can have `educe` replicate the behaviour of `std`'s `derive`'s by using `bound(*)`. See the [`Debug`](#debug) section for more information.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq(bound(*)))]
struct Struct<T> {
    #[educe(PartialEq(ignore))]
    f: T,
}
```

###### Union

The `#[educe(PartialEq(unsafe))]` attribute can be used for a union. The fields of a union cannot be compared with other methods. The implementation is **unsafe** because it disregards the specific fields it utilizes.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq(unsafe))]
union Union {
    f1: u8,
    f2: i32
}
```

#### Eq

Use `#[derive(Educe)]` and `#[educe(Eq)]` to implement the `Eq` trait for a struct, enum, or union. You can also choose to ignore specific fields or set a method to replace the `PartialEq` trait.

###### Basic Usage

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq, Eq)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(PartialEq, Eq)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Ignore Fields

The `ignore` parameter can ignore a specific field.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq, Eq)]
struct Struct {
    #[educe(Eq(ignore))]
    f1: u8
}

#[derive(Educe)]
#[educe(PartialEq, Eq)]
enum Enum {
    V1,
    V2 {
        #[educe(Eq(ignore))]
        f1: u8,
    },
    V3(
        #[educe(Eq(ignore))]
        u8
    ),
}
```

###### Use Another Method to Perform Comparison

The `method` parameter can be utilized to replace the implementation of the `Eq` trait for a field, eliminating the need to implement the `PartialEq` trait for the type of that field.

```rust
use educe::Educe;

fn eq(a: &u8, b: &u8) -> bool {
    a + 1 == *b
}

trait A {
    fn is_same(&self, other: &Self) -> bool;
}

fn eq2<T: A>(a: &T, b: &T) -> bool {
    a.is_same(b)
}

#[derive(Educe)]
#[educe(PartialEq, Eq)]
enum Enum<T: A> {
    V1,
    V2 {
        #[educe(Eq(method(eq)))]
        f1: u8,
    },
    V3(
        #[educe(Eq(method(eq2)))]
        T
    ),
}
```

###### Generic Parameters Bound to the `PartialEq` Trait or Others

Generic parameters will be automatically bound to the `PartialEq` trait if necessary.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq, Eq)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

trait A {
    fn is_same(&self, other: &Self) -> bool;
}

fn eq<T: A>(a: &T, b: &T) -> bool {
    a.is_same(b)
}

#[derive(Educe)]
#[educe(PartialEq(bound(T: std::cmp::PartialEq, K: A)), Eq)]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(Eq(method(eq)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

###### Union

The `#[educe(PartialEq(unsafe), Eq)]` attribute can be used for a union. The fields of a union cannot be compared with other methods. The implementation is **unsafe** because it disregards the specific fields it utilizes.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq(unsafe), Eq)]
union Union {
    f1: u8,
    f2: i32
}
```

#### PartialOrd

Use `#[derive(Educe)]` and `#[educe(PartialOrd)]` to implement the `PartialOrd` trait for a struct or enum. You can also choose to ignore specific fields or set a method to replace the `PartialOrd` trait.

###### Basic Usage

```rust
use educe::Educe;

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
struct Struct {
    f1: u8
}

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Ignore Fields

The `ignore` parameter can ignore a specific field.

```rust
use educe::Educe;

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
struct Struct {
    #[educe(PartialOrd(ignore))]
    f1: u8
}

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
enum Enum {
    V1,
    V2 {
        #[educe(PartialOrd(ignore))]
        f1: u8,
    },
    V3(
        #[educe(PartialOrd(ignore))]
        u8
    ),
}
```

###### Use Another Method to Perform Comparison

The `method` parameter can be utilized to replace the implementation of the `PartialOrd` trait for a field, eliminating the need to implement the `PartialOrd` trait for the type of that field.

```rust
use educe::Educe;

use std::cmp::Ordering;

fn partial_cmp(a: &u8, b: &u8) -> Option<Ordering> {
    if a > b {
        Some(Ordering::Less)
    } else if a < b {
        Some(Ordering::Greater)
    } else {
        Some(Ordering::Equal)
    }
}

trait A {
    fn value(&self) -> u8;
}

fn partial_cmp2<T: A>(a: &T, b: &T) -> Option<Ordering> {
    partial_cmp(&a.value(), &b.value())
}

#[derive(Educe)]
#[educe(PartialEq, PartialOrd)]
enum Enum<T: A> {
    V1,
    V2 {
        #[educe(PartialOrd(method(partial_cmp)))]
        f1: u8,
    },
    V3(
        #[educe(PartialOrd(method(partial_cmp2)))]
        T
    ),
}
```

###### Ranking

Each field can add a `#[educe(PartialOrd(rank = priority_value))]` attribute, where `priority_value` is an integer value indicating its comparison precedence (lower values indicate higher priority). The default `priority_value` for a field depends on its ordinal position (lower towards the front) and starts with `isize::MIN`.

```rust
use educe::Educe;

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
struct Struct {
    #[educe(PartialOrd(rank = 1))]
    f1: u8,
    #[educe(PartialOrd(rank = 0))]
    f2: u8,
}
```

For variants, the discriminant can be explicitly set for comparison.

```rust
use educe::Educe;

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
#[repr(u8)]
enum Enum {
    Three { f1: u8 } = 3,
    Two(u8) = 2,
    One = 1,
}
```

###### Generic Parameters Bound to the `PartialOrd` Trait or Others

Generic parameters will be automatically bound to the `PartialOrd` trait if necessary.

```rust
use educe::Educe;

#[derive(PartialEq, Educe)]
#[educe(PartialOrd)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

use std::cmp::Ordering;

trait A {
    fn value(&self) -> u8;
}

fn partial_cmp<T: A>(a: &T, b: &T) -> Option<Ordering> {
    a.value().partial_cmp(&b.value())
}

#[derive(PartialEq, Educe)]
#[educe(PartialOrd(bound(T: std::cmp::PartialOrd, K: PartialEq + A)))]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(PartialOrd(method(partial_cmp)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

In the above case, `T` is bound to the `PartialOrd` trait, but `K` is not.

You can have `educe` replicate the behaviour of `std`'s `derive`'s by using `bound(*)`. See the [`Debug`](#debug) section for more information.

```rust
use educe::Educe;

#[derive(PartialEq, Educe)]
#[educe(PartialOrd(bound(*)))]
struct Struct<T> {
    #[educe(PartialOrd(ignore))]
    f: T,
}
```

#### Ord

Use `#[derive(Educe)]` and `#[educe(Ord)]` to implement the `Ord` trait for a struct or enum. You can also choose to ignore specific fields or set a method to replace the `Ord` trait.

###### Basic Usage

```rust
use educe::Educe;

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
struct Struct {
    f1: u8
}

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Ignore Fields

The `ignore` parameter can ignore a specific field.

```rust
use educe::Educe;

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
struct Struct {
    #[educe(Ord(ignore))]
    f1: u8
}

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
enum Enum {
    V1,
    V2 {
        #[educe(Ord(ignore))]
        f1: u8,
    },
    V3(
        #[educe(Ord(ignore))]
        u8
    ),
}
```

###### Use Another Method to Perform Comparison

The `method` parameter can be utilized to replace the implementation of the `Ord` trait for a field, eliminating the need to implement the `Ord` trait for the type of that field.

```rust
use educe::Educe;

use std::cmp::Ordering;

fn cmp(a: &u8, b: &u8) -> Ordering {
    if a > b {
        Ordering::Less
    } else if a < b {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

trait A {
    fn value(&self) -> u8;
}

fn cmp2<T: A>(a: &T, b: &T) -> Ordering {
    cmp(&a.value(), &b.value())
}

#[derive(Educe)]
#[educe(PartialEq, Eq, PartialOrd, Ord)]
enum Enum<T: A> {
    V1,
    V2 {
        #[educe(Ord(method(cmp)))]
        f1: u8,
    },
    V3(
        #[educe(Ord(method(cmp2)))]
        T
    ),
}
```

###### Ranking

Each field can add a `#[educe(Ord(rank = priority_value))]` attribute, where `priority_value` is an integer value indicating its comparison precedence (lower values indicate higher priority). The default `priority_value` for a field depends on its ordinal position (lower towards the front) and starts with `isize::MIN`.

```rust
use educe::Educe;

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
struct Struct {
    #[educe(Ord(rank = 1))]
    f1: u8,
    #[educe(Ord(rank = 0))]
    f2: u8,
}
```

For variants, the discriminant can be explicitly set for comparison.

```rust
use educe::Educe;

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
#[repr(u8)]
enum Enum {
    Three { f1: u8 } = 3,
    Two(u8) = 2,
    One = 1,
}
```

###### Generic Parameters Bound to the `Ord` Trait or Others

Generic parameters will be automatically bound to the `Ord` trait if necessary.

```rust
use educe::Educe;

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

use std::cmp::Ordering;

trait A {
    fn value(&self) -> u8;
}

fn cmp<T: A>(a: &T, b: &T) -> Ordering {
    a.value().cmp(&b.value())
}

#[derive(PartialEq, Eq, Educe)]
#[educe(PartialOrd, Ord(bound(T: std::cmp::Ord, K: std::cmp::Ord + A)))]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(PartialOrd(method(cmp)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

#### Hash

Use `#[derive(Educe)]` and `#[educe(Hash)]` to implement the `Hash` trait for a struct, enum, or union. You can also choose to ignore specific fields or set a method to replace the `Hash` trait.

###### Basic Usage

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Hash)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Hash)]
enum Enum {
    V1,
    V2 {
        f1: u8,
    },
    V3(u8),
}
```

###### Ignore Fields

The `ignore` parameter can ignore a specific field.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Hash)]
struct Struct {
    #[educe(Hash(ignore))]
    f1: u8
}

#[derive(Educe)]
#[educe(Hash)]
enum Enum {
    V1,
    V2 {
        #[educe(Hash(ignore))]
        f1: u8,
    },
    V3(
        #[educe(Hash(ignore))]
        u8
    ),
}
```

###### Use Another Method for Hashing

The `method` parameter can be utilized to replace the implementation of the `Hash` trait for a field, eliminating the need to implement the `Hash` trait for the type of that field.

```rust
use educe::Educe;

use std::hash::{Hash, Hasher};

fn hash<H: Hasher>(_s: &u8, state: &mut H) {
    Hash::hash(&100, state)
}

fn hash2<H: Hasher, T>(_s: &T, state: &mut H) {
    Hash::hash(&100, state)
}

#[derive(Educe)]
#[educe(Hash)]
enum Enum<T> {
    V1,
    V2 {
        #[educe(Hash(method(hash)))]
        f1: u8,
    },
    V3(
        #[educe(Hash(method(hash2)))]
        T
    ),
}
```

###### Generic Parameters Bound to the `Hash` Trait or Others

Generic parameters will be automatically bound to the `Hash` trait if necessary.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Hash)]
enum Enum<T, K> {
    V1,
    V2 {
        f1: K,
    },
    V3(
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

use std::hash::{Hash, Hasher};

trait A {
    fn value(&self) -> u8;
}

fn hash<H: Hasher, T: A>(s: &T, state: &mut H) {
    Hash::hash(&s.value(), state)
}

#[derive(Educe)]
#[educe(Hash(bound(T: std::hash::Hash, K: A)))]
enum Enum<T, K> {
    V1,
    V2 {
        #[educe(Hash(method(hash)))]
        f1: K,
    },
    V3(
        T
    ),
}
```

In the above case, `T` is bound to the `Hash` trait, but `K` is not.

You can have `educe` replicate the behaviour of `std`'s `derive`'s by using `bound(*)`. See the [`Debug`](#debug) section for more information.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Hash(bound(*)))]
struct Struct<T> {
    #[educe(Hash(ignore))]
    f: T,
}
```

###### Union

The `#[educe(PartialEq(unsafe), Eq, Hash(unsafe))]` attribute can be used for a union. The fields of a union cannot be hashed with other methods. The implementation is **unsafe** because it disregards the specific fields it utilizes.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(PartialEq(unsafe), Eq, Hash(unsafe))]
union Union {
    f1: u8,
    f2: i32
}
```

#### Default

Use `#[derive(Educe)]` and `#[educe(Default)]` to implement the `Default` trait for a struct, enum, or union. You can also choose to ignore specific fields or set a method to replace the `Hash` trait.

###### Basic Usage

For enums and unions, it is necessary to designate a default variant (for enums) and a default field (for unions) unless the enum has only one variant or the union has only one field.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Default)]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Default)]
enum Enum {
    V1,
    #[educe(Default)]
    V2 {
        f1: u8,
    },
    V3(u8),
}

#[derive(Educe)]
#[educe(Default)]
union Union {
    f1: u8,
    #[educe(Default)]
    f2: f64,
}
```

###### The Default Value for the Entire Type

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Default(expression = Struct { f1: 1 }))]
struct Struct {
    f1: u8
}

#[derive(Educe)]
#[educe(Default(expression = Enum::Struct { f1: 1 }))]
enum Enum {
    Unit,
    Struct {
        f1: u8
    },
    Tuple(u8),
}

#[derive(Educe)]
#[educe(Default(expression = Union { f1: 1 }))]
union Union {
    f1: u8,
    f2: f64,
}
```

You may need to activate the `full` feature to enable support for advanced expressions.

###### The Default Values for Specific Fields

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Default)]
struct Struct {
    #[educe(Default = 1)]
    f1: u8,
    #[educe(Default = 11111111111111111111111111111)]
    f2: i128,
    #[educe(Default = 1.1)]
    f3: f64,
    #[educe(Default = true)]
    f4: bool,
    #[educe(Default = "Hi")]
    f5: &'static str,
    #[educe(Default = "Hello")]
    f6: String,
    #[educe(Default = 'M')]
    f7: char,
}

#[derive(Educe)]
#[educe(Default)]
enum Enum {
    Unit,
    #[educe(Default)]
    Tuple(
        #[educe(Default(expression = 0 + 1))]
        u8,
        #[educe(Default(expression = -11111111111111111111111111111 * -1))]
        i128,
        #[educe(Default(expression = 1.0 + 0.1))]
        f64,
        #[educe(Default(expression = !false))]
        bool,
        #[educe(Default(expression = "Hi"))]
        &'static str,
        #[educe(Default(expression = String::from("Hello")))]
        String,
        #[educe(Default(expression = 'M'))]
        char,
    ),
}

#[derive(Educe)]
#[educe(Default)]
union Union {
    f1: u8,
    f2: i128,
    f3: f64,
    f4: bool,
    #[educe(Default = "Hi")]
    f5: &'static str,
    f6: char,
}
```

###### Generic Parameters Bound to the `Default` Trait or Others

Generic parameters will be automatically bound to the `Default` trait if necessary.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Default)]
enum Enum<T> {
    Unit,
    #[educe(Default)]
    Struct {
        f1: T
    },
    Tuple(T),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Default(bound(T: std::default::Default)))]
enum Enum<T> {
    Unit,
    #[educe(Default)]
    Struct {
        f1: T
    },
    Tuple(T),
}
```

###### The `new` Associated Function

With the `#[educe(Default(new))]` attribute, your type will include an additional associated function called `new`. This function can be utilized to invoke the `default` method of the `Default` trait.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Default(new))]
struct Struct {
    f1: u8
}
```

#### Deref

Use `#[derive(Educe)]` and `#[educe(Deref)]` to implement the `Deref` trait for a struct or enum.

###### Basic Usage

You must designate a field as the default for obtaining an immutable reference unless the number of fields is exactly one.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Deref)]
struct Struct {
    f1: u8,
    #[educe(Deref)]
    f2: u8,
}

#[derive(Educe)]
#[educe(Deref)]
enum Enum {
    Struct {
        f1: u8
    },
    Struct2 {
        f1: u8,
        #[educe(Deref)]
        f2: u8,
    },
    Tuple(u8),
    Tuple2(
        u8,
        #[educe(Deref)]
        u8
    ),
}
```

#### DerefMut

Use `#[derive(Educe)]` and `#[educe(DerefMut)]` to implement the `DerefMut` trait for a struct or enum.

###### Basic Usage

You must designate a field as the default for obtaining an mutable reference unless the number of fields is exactly one.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Deref, DerefMut)]
struct Struct {
    f1: u8,
    #[educe(Deref, DerefMut)]
    f2: u8,
}

#[derive(Educe)]
#[educe(Deref, DerefMut)]
enum Enum {
    Struct {
        f1: u8
    },
    Struct2 {
        f1: u8,
        #[educe(Deref, DerefMut)]
        f2: u8,
    },
    Tuple(u8),
    Tuple2(
        #[educe(DerefMut)]
        u8,
        #[educe(Deref)]
        u8
    ),
}
```

The mutable dereferencing fields do not need to be the same as the immutable dereferencing fields, but their types must be consistent.

#### Into

Use `#[derive(Educe)]` and `#[educe(Into(type))]` to implement the `Into<type>` trait for a struct or enum.

###### Basic Usage

You need to designate a field as the default for `Into<type>` conversion unless the number of fields is exactly one. If you don't, educe will automatically try to find a proper one.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Into(u8), Into(u16))]
struct Struct {
    f1: u8,
    f2: u16,
}

#[derive(Educe)]
#[educe(Into(u8))]
enum Enum {
    V1 {
        f1: u8,
        #[educe(Into(u8))]
        f2: u8,
    },
    V2 (
        u8
    ),
}
```

###### Use Another Method to Perform Into Conversion

The `method` parameter can be utilized to replace the implementation of the `Into` trait for a field, eliminating the need to implement the `Into` trait for the type of that field.

```rust
use educe::Educe;

fn into(v: u16) -> u8 {
    v as u8
}

#[derive(Educe)]
#[educe(Into(u8))]
enum Enum {
    V1 {
        #[educe(Into(u8, method(into)))]
        f1: u16,
    },
    V2 (
        u8
    ),
}
```

###### Generic Parameters Bound to the `Into` Trait or Others

Generic parameters will be automatically bound to the `Into<type>` trait if necessary.

```rust
use educe::Educe;

#[derive(Educe)]
#[educe(Into(u8))]
enum Enum<T, K> {
    V1 {
        f1: K,
    },
    V2 (
        T
    ),
}
```

Or you can set the where predicates by yourself.

```rust
use educe::Educe;

fn into<T>(_v: T) -> u8 {
    0
}

#[derive(Educe)]
#[educe(Into(u8, bound(K: Into<u8>)))]
enum Enum<T, K> {
    V1 {
        f1: K,
    },
    V2 (
        #[educe(Into(u8, method(into)))]
        T
    ),
}
```

## Crates.io

https://crates.io/crates/educe

## Documentation

https://docs.rs/educe

## License

[MIT](LICENSE)
