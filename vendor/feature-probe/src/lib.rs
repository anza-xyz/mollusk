#![doc(html_root_url = "https://docs.rs/feature-probe/0.1.1")]
//! To support multiple versions of Rust, it's often necessary to conditionally
//! compile parts of our libraries or programs. It's possible to allow users to
//! specify what features to enable, but detection is better, because users get
//! all the features that their version of Rust supports. And while we could check
//! the rustc version, it's better to probe for individual features. That way,
//! code will work both on nightly, and on stable releases after particular features
//! stabilize, without changes.
//!
//! ## Usage
//!
//! It’s [on crates.io](https://crates.io/crates/feature-probe), so you can add
//!
//! ```toml
//! [build-dependencies]
//! feature-probe = "0.1.1"
//! ```
//!
//! Then add to your `build.rs`:
//!
//! ```no_compile
//! extern crate feature_probe;
//!
//! use feature_probe::Probe;
//! ```
//!
//! Then you can probe for features such as types or expressions. For example:
//!
//! ```no_compile
//! fn main () {
//!     let probe = Probe::new();
//!
//!     if probe.probe_type("i128") {
//!         println!("cargo:rustc-cfg=int_128");
//!     }
//!
//!     if probe.probe_type("::std::ops::RangeInclusive<u64>") {
//!         println!("cargo:rustc-cfg=inclusive_range");
//!     }
//! }
//! ```
//!
//! This crate supports Rust version 1.16.0 and later.

use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::{Command, Stdio};

/// A probe object, which is used for probing for features.
///
/// Create this with [`ProbeProbeo::new`](#method.new), and then probe with
/// one of the probing methods.
#[derive(Debug)]
pub struct Probe {
    rustc:   OsString,
    out_dir: OsString,
}

impl Probe {
    /// Creates a new [`Probe`](struct.Probe.html) object with a default
    /// configuration.
    ///
    /// In particular, it consults the environment variable `"RUSTC"` to determine
    /// what Rust compiler to use, and the environment variable `"OUT_DIR"` to
    /// determine where to put object files. If these are not set, they default to
    /// the values `"rustc"` and `"target"`, respectively.
    ///
    /// # Panics
    ///
    /// If the child `rustc` cannot be started or communicated with.
    ///
    /// # Examples
    ///
    /// ```
    /// use feature_probe::Probe;
    ///
    /// let probe = Probe::new();
    /// assert!( probe.probe_type("u32") );
    /// ```
    pub fn new() -> Self {
        Probe {
            rustc:   env_var_or("RUSTC",   "rustc"),
            out_dir: env_var_or("OUT_DIR", "target"),
        }
    }

    /// Probes for the existence of the given type by name.
    ///
    /// # Panics
    ///
    /// If the child `rustc` cannot be started or communicated with.
    ///
    /// # Examples
    ///
    /// ```
    /// use feature_probe::Probe;
    ///
    /// let probe = Probe::new();
    /// assert!(   probe.probe_type("u32") );
    /// assert!( ! probe.probe_type("u512") );
    /// ```
    pub fn probe_type(&self, type_name: &str) -> bool {
        self.probe(&format!("pub type T = {}; fn main() {{ }}", type_name))
    }

    /// Probes whether the given expression can be compiled.
    ///
    /// # Examples
    ///
    /// ```
    /// use feature_probe::Probe;
    ///
    /// let probe = Probe::new();
    /// assert!(   probe.probe_expression("3 + 4") );
    /// assert!( ! probe.probe_expression("3 + true") );
    pub fn probe_expression(&self, expression: &str) -> bool {
        self.probe(&format!("fn main() {{ {}; }}", expression))
    }

    /// Probes for whether a whole program can be compiled.
    ///
    /// # Panics
    ///
    /// If the child `rustc` cannot be started or communicated with.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate feature_probe;
    /// # fn main() {
    /// use feature_probe::Probe;
    ///
    /// let probe = Probe::new();
    /// assert!(   probe.probe("fn main() { }") );
    /// assert!( ! probe.probe("fn main(args: Vec<String>) { }") );
    /// # }
    /// ```
    pub fn probe(&self, code: &str) -> bool {
        self.probe_result(code).expect("Probe::probe")
    }

    /// Probes for whether a whole program can be compiled.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate feature_probe;
    /// # fn main() {
    /// use feature_probe::Probe;
    ///
    /// let probe = Probe::new();
    /// assert_eq!( probe.probe_result("fn main() { }").unwrap(),                  true );
    /// assert_eq!( probe.probe_result("fn main(args: Vec<String>) { }").unwrap(), false );
    /// # }
    /// ```
    pub fn probe_result(&self, code: &str) -> io::Result<bool> {
        let mut child = Command::new(&self.rustc)
            .arg("--out-dir")
            .arg(&self.out_dir)
            .arg("--emit=obj")
            .arg("-")
            .stdin(Stdio::piped())
            .spawn()?;

        child
            .stdin
            .as_mut().unwrap()
            .write_all(code.as_bytes())?;

        Ok(child.wait()?.success())
    }
}

impl Default for Probe {
    fn default() -> Self {
        Probe::new()
    }
}

fn env_var_or(var: &str, default: &str) -> OsString {
    env::var_os(var).unwrap_or_else(|| default.into())
}
