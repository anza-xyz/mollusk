use core::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    ops::Neg,
    str::FromStr,
};

#[derive(Debug, Copy, Eq, Clone)]
pub(crate) enum Int128 {
    Signed(i128),
    Unsigned(u128),
}

impl PartialEq for Int128 {
    #[inline]
    fn eq(&self, other: &Int128) -> bool {
        match self {
            Self::Signed(i) => match other {
                Self::Signed(i2) => i.eq(i2),
                Self::Unsigned(u2) => {
                    if i.is_negative() {
                        false
                    } else {
                        (*i as u128).eq(u2)
                    }
                },
            },
            Self::Unsigned(u) => match other {
                Self::Signed(i2) => {
                    if i2.is_negative() {
                        false
                    } else {
                        u.eq(&(*i2 as u128))
                    }
                },
                Self::Unsigned(u2) => u.eq(u2),
            },
        }
    }
}

impl PartialOrd for Int128 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Int128 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            Self::Signed(i) => match other {
                Self::Signed(i2) => i.cmp(i2),
                Self::Unsigned(u2) => {
                    if i.is_negative() {
                        Ordering::Less
                    } else {
                        (*i as u128).cmp(u2)
                    }
                },
            },
            Self::Unsigned(u) => match other {
                Self::Signed(i2) => {
                    if i2.is_negative() {
                        Ordering::Greater
                    } else {
                        u.cmp(&(*i2 as u128))
                    }
                },
                Self::Unsigned(u2) => u.cmp(u2),
            },
        }
    }
}

impl Display for Int128 {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Signed(i) => Display::fmt(i, f),
            Self::Unsigned(u) => Display::fmt(u, f),
        }
    }
}

impl Default for Int128 {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl Int128 {
    pub(crate) const ZERO: Self = Self::Unsigned(0);
}

macro_rules! impl_from_signed {
    (@inner $t: ty) => {
        impl From<$t> for Int128 {
            #[inline]
            fn from(value: $t) -> Self {
                Int128::Signed(value as i128)
            }
        }
    };
    ($($t: ty),+ $(,)*) => {
        $(
            impl_from_signed!(@inner $t);
        )*
    };
}

impl_from_signed!(i8, i16, i32, i64, i128, isize);

impl FromStr for Int128 {
    type Err = ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('-') {
            Ok(Self::Signed(s.parse()?))
        } else {
            Ok(Self::Unsigned(s.parse()?))
        }
    }
}

impl Neg for Int128 {
    type Output = Int128;

    fn neg(self) -> Self::Output {
        match self {
            Self::Signed(i) => {
                if i == i128::MIN {
                    Self::Unsigned(1 << 127)
                } else {
                    Self::Signed(-i)
                }
            },
            Self::Unsigned(u) => match u.cmp(&(1 << 127)) {
                Ordering::Equal => Self::Signed(i128::MIN),
                Ordering::Less => Self::Signed(-(u as i128)),
                Ordering::Greater => panic!("-{} is experiencing an overflow", u),
            },
        }
    }
}

impl Int128 {
    #[inline]
    pub(crate) fn inc(&mut self) {
        match self {
            Self::Signed(i) => {
                if *i == i128::MAX {
                    *self = Self::Unsigned(1 << 127)
                } else {
                    *i += 1;
                }
            },
            Self::Unsigned(u) => {
                *u = u.saturating_add(1);
            },
        }
    }
}
