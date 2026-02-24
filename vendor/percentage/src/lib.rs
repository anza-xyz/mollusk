//! # Percentage
//!
//! `percentage` is a crate trying to make using percentages in a safer way and easier to debug.
//! Whenever you see a Percentage, you will know what is being calculated, instead of having to revise the code.
//!
//! # Example
//!
//! ```
//! // You only need to import the `Percentage` struct
//! use percentage::Percentage;
//!
//! // Here we create the percentage to apply
//! let percent = Percentage::from(50);
//!
//! println!("{}", percent.value()); // Will print '50'
//!
//! // We can apply the percent to any number we want
//! assert_eq!(15, percent.apply_to(30));
//! println!("50% of 30 is: {}", percent.apply_to(30)); // Will print '50% of 30 is: 15'
//!
//! // If you need to use floating points for the percent, you can use `from_decimal` instead
//!
//! let percent = Percentage::from_decimal(0.5);
//! assert_eq!(15.0, percent.apply_to(30.0));
//! println!("50% of 30.0 is: {}", percent.apply_to(30.0)); // Will print '50% of 30.0 is: 15.0'
//!
//! ```

extern crate num;

use num::{Num, NumCast};

pub struct PercentageInteger {
    value: u8,
}

pub struct PercentageDecimal {
    value: f64,
}

impl PercentageInteger {
    /// Returns the percentage applied to the number given.
    ///
    /// # Arguments
    ///
    /// * `value` - The number to apply the percentage.
    ///
    /// # Examples
    ///
    /// ```
    /// use percentage::Percentage;
    ///
    /// let number = 90;
    /// let percentage = Percentage::from(50);
    ///
    /// assert_eq!(45, percentage.apply_to(number));
    /// ```
    pub fn apply_to<T: Num + Ord + Copy + NumCast>(&self, value: T) -> T {
        (value * NumCast::from(self.value).unwrap()) / NumCast::from(100).unwrap()
    }

    /// Returns the percentage saved.
    ///
    /// # Examples
    ///
    /// ```
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from(50);
    ///
    /// assert_eq!(50, percentage.value());
    /// ```
    pub fn value(&self) -> u8 {
        self.value
    }
}

impl PercentageDecimal {
    /// Returns the percentage applied to the f64 given.
    ///
    /// # Arguments
    ///
    /// * `value` - The number to apply the percentage.
    ///
    /// # Examples
    ///
    /// ```
    /// use percentage::Percentage;
    ///
    /// let number = 90.0;
    /// let percentage = Percentage::from_decimal(0.5);
    ///
    /// assert_eq!(45.0, percentage.apply_to(number));
    /// ```
    pub fn apply_to(&self, value: f64) -> f64 {
        value * self.value
    }

    /// Returns the percentage saved.
    ///
    /// # Examples
    ///
    /// ```
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from_decimal(0.5);
    ///
    /// assert_eq!(0.5, percentage.value());
    /// ```
    pub fn value(&self) -> f64 {
        self.value
    }
}

pub struct Percentage;

impl Percentage {
    /// Returns a new `PercentageInteger` with the Given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The number to use as the percentage between 0 and 100.
    ///
    /// # Example
    /// ```
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from(50);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `value` is over 100
    /// ```rust,should_panic
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from(150);
    /// ```
    ///
    /// Panics if `value` is below 0
    /// ```rust,should_panic
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from(-150);
    /// ```
    pub fn from<T: Num + Ord + Copy + NumCast>(value: T) -> PercentageInteger {
        let value: u8 = NumCast::from(value)
            .unwrap_or_else(|| panic!("Percentage value must be between 0 and 100"));
        if value > 100 {
            panic!("Percentage value must be between 0 and 100");
        }
        PercentageInteger { value }
    }

    /// Returns a new `PercentageDecimal` with the Given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The number to use as the percentage between 0.0 and 1.0.
    ///
    /// # Example
    /// ```
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from_decimal(0.5);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `value` is over 1.0
    /// ```rust,should_panic
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from_decimal(1.5);
    /// ```
    ///
    /// Panics if `value` is below 0
    /// ```rust,should_panic
    /// use percentage::Percentage;
    ///
    /// let percentage = Percentage::from_decimal(-1.5);
    /// ```
    pub fn from_decimal(value: f64) -> PercentageDecimal {
        if value < 0.0 || value > 1.0 {
            panic!("Percentage value must be between 0 and 1");
        }
        PercentageDecimal { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_panic]
    fn from_should_panic_if_value_is_over_100() {
        Percentage::from(101);
    }
    #[test]
    #[should_panic]
    fn from_should_panic_if_value_is_below_0() {
        Percentage::from(-1);
    }
    #[test]
    fn from_should_save_value_on_u8_format() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15).value);
    }
    #[test]
    fn from_should_save_value_from_i8_or_u8() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i8).value);
        assert_eq!(test, Percentage::from(15 as u8).value);
    }
    #[test]
    fn from_should_save_value_from_i16_or_u16() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i16).value);
        assert_eq!(test, Percentage::from(15 as u16).value);
    }
    #[test]
    fn from_should_save_value_from_i32_or_u32() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i32).value);
        assert_eq!(test, Percentage::from(15 as u32).value);
    }
    #[test]
    fn from_should_save_value_from_i64_or_u64() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i64).value);
        assert_eq!(test, Percentage::from(15 as u64).value);
    }
    #[test]
    fn from_should_save_value_from_i128_or_u128() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i128).value);
        assert_eq!(test, Percentage::from(15 as u128).value);
    }
    #[test]
    fn from_should_save_value_from_isize_or_usize() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as isize).value);
        assert_eq!(test, Percentage::from(15 as usize).value);
    }

    #[test]
    #[should_panic]
    fn from_decimal_should_panic_if_value_is_over_1() {
        Percentage::from_decimal(1.1);
    }
    #[test]
    #[should_panic]
    fn from_decimal_should_panic_if_value_is_below_0() {
        Percentage::from_decimal(-1.1);
    }
    #[test]
    fn from_decimal_should_save_value_from_f64() {
        let test: f64 = 0.34567;
        assert_eq!(test, Percentage::from_decimal(0.34567 as f64).value);
    }

    #[test]
    fn value_should_return_decimal_percentage_on_f64() {
        let test: f64 = 0.34;
        assert_eq!(test, Percentage::from_decimal(0.34).value());
    }
    #[test]
    fn value_should_return_integer_percentage_on_u8() {
        let test: u8 = 34;
        assert_eq!(test, Percentage::from(34 as i32).value());
    }

    #[test]
    fn apply_to_should_return_the_value_with_the_percentage_applied_with_the_same_type() {
        let number = 100;
        assert_eq!(84, Percentage::from(84).apply_to(number));
        assert_eq!(84 as i32, Percentage::from(84).apply_to(number as i32));
        assert_eq!(84 as u32, Percentage::from(84).apply_to(number as u32));
        assert_eq!(84.0, Percentage::from_decimal(0.84).apply_to(number as f64));
    }
}
