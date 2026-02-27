pub use std::ops::{Range, RangeTo, RangeFrom, RangeFull};

#[cfg(inclusive_range)]
pub use std::ops::{RangeInclusive, RangeToInclusive};

#[cfg(inclusive_range)]
pub fn get_inclusive_bounds(range: RangeInclusive<u64>) -> Option<(u64, u64)> {
    let mut r1 = range.clone();
    let mut r2 = range;
    Some((r1.next()?, r2.next_back()?))
}

