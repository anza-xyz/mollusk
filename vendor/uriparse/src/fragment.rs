//! Fragment Component
//!
//! See [[RFC3986, Section 3.5](https://tools.ietf.org/html/rfc3986#section-3.5)].

use std::borrow::Cow;
use std::convert::{Infallible, TryFrom};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str;

use crate::utility::{
    get_percent_encoded_value, normalize_string, percent_encoded_equality, percent_encoded_hash,
    UNRESERVED_CHAR_MAP,
};

/// A map of byte characters that determines if a character is a valid fragment character.
#[rustfmt::skip]
const FRAGMENT_CHAR_MAP: [u8; 256] = [
 // 0     1     2     3     4     5     6     7     8     9     A     B     C     D     E     F
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 0
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 1
    0, b'!',    0,    0, b'$', b'%', b'&',b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.', b'/', // 2
 b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b':', b';',    0, b'=',    0, b'?', // 3
 b'@', b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', // 4
 b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z',    0,    0,    0,    0, b'_', // 5
    0, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', // 6
 b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z',    0,    0,    0, b'~',    0, // 7
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 8
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 9
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // A
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // B
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // C
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // D
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // E
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // F
];

/// The fragment component as defined in
/// [[RFC3986, Section 3.5](https://tools.ietf.org/html/rfc3986#section-3.5)].
///
/// The fragment is case-sensitive. Furthermore, percent-encoding plays no role in equality checking
/// for characters in the unreserved character set meaning that `"fragment"` and `"fr%61gment"` are
/// identical. Both of these attributes are reflected in the equality and hash functions.
///
/// However, be aware that just because percent-encoding plays no role in equality checking does not
/// mean that the fragment is normalized. If the fragment needs to be normalized, use the
/// [`Fragment::normalize`] function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fragment<'fragment> {
    /// The internal fragment source that is either owned or borrowed.
    fragment: Cow<'fragment, str>,

    /// Whether the fragment is normalized.
    normalized: bool,
}

impl Fragment<'_> {
    /// Returns a new fragment which is identical but has a lifetime tied to this fragment.
    pub fn as_borrowed(&self) -> Fragment {
        use self::Cow::*;

        let fragment = match &self.fragment {
            Borrowed(borrowed) => *borrowed,
            Owned(owned) => owned.as_str(),
        };

        Fragment {
            fragment: Cow::Borrowed(fragment),
            normalized: self.normalized,
        }
    }

    /// Returns a `str` representation of the fragment.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Fragment;
    ///
    /// let fragment = Fragment::try_from("fragment").unwrap();
    /// assert_eq!(fragment.as_str(), "fragment");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.fragment
    }

    /// Converts the [`Fragment`] into an owned copy.
    ///
    /// If you construct the fragment from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the fragment will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Fragment<'static> {
        Fragment {
            fragment: Cow::from(self.fragment.into_owned()),
            normalized: self.normalized,
        }
    }

    /// Returns whether the fragment is normalized.
    ///
    /// A normalized fragment will have no bytes that are in the unreserved character set
    /// percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Fragment;
    ///
    /// let fragment = Fragment::try_from("fragment").unwrap();
    /// assert!(fragment.is_normalized());
    ///
    /// let mut fragment = Fragment::try_from("%ff%ff").unwrap();
    /// assert!(!fragment.is_normalized());
    /// fragment.normalize();
    /// assert!(fragment.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    /// Normalizes the fragment such that it will have no bytes that are in the unreserved character
    /// set percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// If the fragment is already normalized, the function will return immediately. Otherwise, if
    /// the fragment is not owned, this function will perform an allocation to clone it. The
    /// normalization itself though, is done in-place with no extra memory allocations required.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Fragment;
    ///
    /// let mut fragment = Fragment::try_from("fragment").unwrap();
    /// fragment.normalize();
    /// assert_eq!(fragment, "fragment");
    ///
    /// let mut fragment = Fragment::try_from("%ff%41").unwrap();
    /// assert_eq!(fragment, "%ff%41");
    /// fragment.normalize();
    /// assert_eq!(fragment, "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if !self.normalized {
            // Unsafe: Fragments must be valid ASCII-US, so this is safe.
            unsafe { normalize_string(&mut self.fragment.to_mut(), true) };
            self.normalized = true;
        }
    }
}

impl AsRef<[u8]> for Fragment<'_> {
    fn as_ref(&self) -> &[u8] {
        self.fragment.as_bytes()
    }
}

impl AsRef<str> for Fragment<'_> {
    fn as_ref(&self) -> &str {
        &self.fragment
    }
}

impl Deref for Fragment<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.fragment
    }
}

impl Display for Fragment<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(&self.fragment)
    }
}

impl Eq for Fragment<'_> {}

impl<'fragment> From<Fragment<'fragment>> for String {
    fn from(value: Fragment<'fragment>) -> Self {
        value.to_string()
    }
}

impl Hash for Fragment<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        percent_encoded_hash(self.fragment.as_bytes(), state, true);
    }
}

impl PartialEq for Fragment<'_> {
    fn eq(&self, other: &Fragment) -> bool {
        *self == *other.as_bytes()
    }
}

impl PartialEq<[u8]> for Fragment<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        percent_encoded_equality(self.fragment.as_bytes(), other, true)
    }
}

impl<'fragment> PartialEq<Fragment<'fragment>> for [u8] {
    fn eq(&self, other: &Fragment<'fragment>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for Fragment<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'fragment> PartialEq<Fragment<'fragment>> for &'a [u8] {
    fn eq(&self, other: &Fragment<'fragment>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for Fragment<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'fragment> PartialEq<Fragment<'fragment>> for str {
    fn eq(&self, other: &Fragment<'fragment>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Fragment<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'fragment> PartialEq<Fragment<'fragment>> for &'a str {
    fn eq(&self, other: &Fragment<'fragment>) -> bool {
        other == self.as_bytes()
    }
}

impl<'fragment> TryFrom<&'fragment [u8]> for Fragment<'fragment> {
    type Error = FragmentError;

    fn try_from(value: &'fragment [u8]) -> Result<Self, Self::Error> {
        let mut bytes = value.iter();
        let mut normalized = true;

        while let Some(&byte) = bytes.next() {
            match FRAGMENT_CHAR_MAP[byte as usize] {
                0 => return Err(FragmentError::InvalidCharacter),
                b'%' => {
                    match get_percent_encoded_value(bytes.next().cloned(), bytes.next().cloned()) {
                        Ok((hex_value, uppercase)) => {
                            if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                                normalized = false;
                            }
                        }
                        Err(_) => return Err(FragmentError::InvalidPercentEncoding),
                    }
                }
                _ => (),
            }
        }

        // Unsafe: The loop above makes sure the byte string is valid ASCII-US.
        Ok(Fragment {
            fragment: Cow::from(unsafe { str::from_utf8_unchecked(value) }),
            normalized,
        })
    }
}

impl<'fragment> TryFrom<&'fragment str> for Fragment<'fragment> {
    type Error = FragmentError;

    fn try_from(value: &'fragment str) -> Result<Self, Self::Error> {
        Fragment::try_from(value.as_bytes())
    }
}

/// An error representing an invalid fragment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum FragmentError {
    /// The fragment contained an invalid character.
    InvalidCharacter,

    /// The fragment contained an invalid percent encoding (e.g. `"%ZZ"`).
    InvalidPercentEncoding,
}

impl Display for FragmentError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::FragmentError::*;

        match self {
            InvalidCharacter => write!(formatter, "invalid fragment character"),
            InvalidPercentEncoding => write!(formatter, "invalid fragment percent encoding"),
        }
    }
}

impl Error for FragmentError {}

impl From<Infallible> for FragmentError {
    fn from(_: Infallible) -> Self {
        FragmentError::InvalidCharacter
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fragment_normalize() {
        fn test_case(value: &str, expected: &str) {
            let mut fragment = Fragment::try_from(value).unwrap();
            fragment.normalize();
            assert_eq!(fragment, expected);
        }

        test_case("", "");
        test_case("%ff", "%FF");
        test_case("%41", "A");
    }

    #[test]
    fn test_fragment_parse() {
        use self::FragmentError::*;

        assert_eq!(Fragment::try_from("").unwrap(), "");
        assert_eq!(Fragment::try_from("fragment").unwrap(), "fragment");
        assert_eq!(Fragment::try_from("fRaGmEnT").unwrap(), "fRaGmEnT");
        assert_eq!(Fragment::try_from("%ff%ff%ff%41").unwrap(), "%ff%ff%ff%41");

        assert_eq!(Fragment::try_from(" "), Err(InvalidCharacter));
        assert_eq!(Fragment::try_from("%"), Err(InvalidPercentEncoding));
        assert_eq!(Fragment::try_from("%f"), Err(InvalidPercentEncoding));
        assert_eq!(Fragment::try_from("%zz"), Err(InvalidPercentEncoding));
    }
}
