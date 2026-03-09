# feature-probe-rs: probe for rustc features from `build.rs`

[![Build Status](https://travis-ci.org/tov/feature-probe-rs.svg?branch=master)](https://travis-ci.org/tov/feature-probe-rs)
[![Crates.io](https://img.shields.io/crates/v/feature-probe.svg?maxAge=2592000)](https://crates.io/crates/feature-probe)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE-APACHE)

To support multiple versions of Rust, it's often necessary to conditionally
compile parts of our libraries or programs. It's possible to allow users to
specify what features to enable, but detection is better, because users get
all the features that their version of Rust supports. And while we could check
the rustc version, it's better to probe for individual features. That way,
code will work both on nightly, and on stable releases after particular features
stabilize, without changes.

## Usage

It’s [on crates.io](https://crates.io/crates/feature-probe), so you can add

```toml
[build-dependencies]
feature-probe = "0.1.1"
```

Then add to your `build.rs`:

```rust
extern crate feature_probe;

use feature_probe::Probe;
```

Then you can probe for features such as types or expressions. For example:

```rust
fn main () {
    let probe = Probe::new();
    
    if probe.probe_type("i128") {
        println!("cargo:rustc-cfg=int_128");
    }
    
    if probe.probe_type("::std::ops::RangeInclusive<u64>") {
        println!("cargo:rustc-cfg=inclusive_range");
    }
}
```

This crate supports Rust version 1.16.0 and later.
