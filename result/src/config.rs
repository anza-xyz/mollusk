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
                false
            }
        } else {
            true
        }
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

#[cfg(test)]
mod tests {
    use super::Config;

    fn compare_all(config: &Config, pairs: &[(u64, u64)], compared: &mut usize) -> bool {
        let mut pass = true;
        for (left, right) in pairs {
            *compared += 1;
            pass &= compare!(config, "value", left, right);
        }
        pass
    }

    #[test]
    fn compare_keeps_checking_after_a_mismatch() {
        let config = Config {
            panic: false,
            ..Config::default()
        };
        let mut compared = 0;
        assert!(!compare_all(
            &config,
            &[(1, 2), (3, 4), (5, 5)],
            &mut compared
        ));
        assert_eq!(compared, 3);
    }
}
