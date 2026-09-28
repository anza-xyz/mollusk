//! Configuration and context for result validation.

pub struct Config {
    /// Panic on harness error. If disabled, checks return a result instead.
    pub panic: bool,
    /// Enable verbose output for checks.
    pub verbose: bool,
    /// Assert that no account is left in a rent state the runtime would reject
    /// with `InsufficientFundsForRent`.
    pub rent_exempt_checks: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panic: true,
            verbose: false,
            rent_exempt_checks: true,
        }
    }
}

macro_rules! compare {
    ($c:expr, $check:expr, $left:expr, $right:expr $(,)?) => {{
        if $left != $right {
            let msg = format!(
                "CHECK FAILED: {}\n  Expected: `{:?}`,\n Got: `{:?}`",
                $check, $left, $right
            );
            if $c.panic {
                panic!("{}", msg);
            } else {
                if $c.verbose {
                    println!("{}", msg);
                }
                return false;
            }
        }
        true
    }};
}

macro_rules! throw {
    ($c:expr, $($arg:tt)+) => {{
        let msg = format!($($arg)+);
        if $c.panic {
            panic!("{}", msg);
        } else {
            if $c.verbose {
                eprintln!("{}", msg);
            }
        }
        false
    }};
}

pub(crate) use {compare, throw};
