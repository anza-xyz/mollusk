//! Definitions for the native SOL token and its fractional lamports.

#![allow(clippy::arithmetic_side_effects)]

/// There are 10^9 lamports in one SOL
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const SOL_DECIMALS: usize = 9;

/// Convert native tokens (SOL) into fractional native tokens (lamports)
pub fn sol_str_to_lamports(sol_str: &str) -> Option<u64> {
    if sol_str == "." {
        None
    } else {
        let (sol, lamports) = sol_str.split_once('.').unwrap_or((sol_str, ""));
        let sol = if sol.is_empty() {
            0
        } else {
            sol.parse::<u64>().ok()?
        };
        let lamports = if lamports.is_empty() {
            0
        } else {
            format!("{lamports:0<9}")[..SOL_DECIMALS].parse().ok()?
        };
        LAMPORTS_PER_SOL
            .checked_mul(sol)
            .and_then(|x| x.checked_add(lamports))
    }
}

use std::fmt::{Debug, Display, Formatter, Result};
pub struct Sol(pub u64);

impl Sol {
    fn write_in_sol(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "◎{}.{:09}",
            self.0 / LAMPORTS_PER_SOL,
            self.0 % LAMPORTS_PER_SOL
        )
    }
}

impl Display for Sol {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.write_in_sol(f)
    }
}

impl Debug for Sol {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.write_in_sol(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sol_str_to_lamports() {
        assert_eq!(0, sol_str_to_lamports("0.0").unwrap());
        assert_eq!(1, sol_str_to_lamports("0.000000001").unwrap());
        assert_eq!(10, sol_str_to_lamports("0.00000001").unwrap());
        assert_eq!(100, sol_str_to_lamports("0.0000001").unwrap());
        assert_eq!(1000, sol_str_to_lamports("0.000001").unwrap());
        assert_eq!(10000, sol_str_to_lamports("0.00001").unwrap());
        assert_eq!(100000, sol_str_to_lamports("0.0001").unwrap());
        assert_eq!(1000000, sol_str_to_lamports("0.001").unwrap());
        assert_eq!(10000000, sol_str_to_lamports("0.01").unwrap());
        assert_eq!(100000000, sol_str_to_lamports("0.1").unwrap());
        assert_eq!(1000000000, sol_str_to_lamports("1").unwrap());
        assert_eq!(4_100_000_000, sol_str_to_lamports("4.1").unwrap());
        assert_eq!(8_200_000_000, sol_str_to_lamports("8.2").unwrap());
        assert_eq!(8_502_282_880, sol_str_to_lamports("8.50228288").unwrap());

        assert_eq!(
            u64::MAX,
            sol_str_to_lamports("18446744073.709551615").unwrap()
        );
        // bigger than u64::MAX, error
        assert_eq!(None, sol_str_to_lamports("18446744073.709551616"));
        // Negative, error
        assert_eq!(None, sol_str_to_lamports("-0.000000001"));
        // i64::MIN as string, error
        assert_eq!(None, sol_str_to_lamports("-9223372036.854775808"));
    }
}
