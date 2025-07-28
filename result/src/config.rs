//! Configuration and context for result validation.

use {solana_pubkey::Pubkey, solana_rent::Rent};

pub struct Config {
    pub panic: bool,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panic: true,
            verbose: false,
        }
    }
}

/// A trait for providing context to the checks.
///
/// Developers who run checks on standalone results, rather than passing checks
/// directly to methods like `Mollusk::process_and_validate_instruction`, may
/// wish to customize the context in which the checks are run. For example,
/// one may wish to evaluate resulting account lamports with a custom `Rent`
/// configuration. This trait allows such customization.
pub trait CheckContext {
    fn is_rent_exempt(&self, lamports: u64, space: usize, owner: Pubkey) -> bool {
        owner.eq(&Pubkey::default()) && lamports == 0 || Rent::default().is_exempt(lamports, space)
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
