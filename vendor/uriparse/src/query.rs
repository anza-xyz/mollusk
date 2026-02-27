//! Query Component
//!
//! See [[RFC3986, Section 3.4](https://tools.ietf.org/html/rfc3986#section-3.4)].
//!
//! This crate does not do query string parsing, it will simply make sure that it is a valid query
//! string as defined by the RFC. You will need to use another crate (e.g.
//! [queryst](https://github.com/rustless/queryst)) if you want it parsed.

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

/// A map of byte characters that determines if a character is a valid query character.
#[rustfmt::skip]
const QUERY_CHAR_MAP: [u8; 256] = [
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

/// The query component as defined in
/// [[RFC3986, Section 3.4](https://tools.ietf.org/html/rfc3986#section-3.4)].
///
/// The query is case-sensitive. Furthermore, percent-encoding plays no role in equality checking
/// for characters in the unreserved character set meaning that `"query"` and `"que%72y"` are
/// identical. Both of these attributes are reflected in the equality and hash functions.
///
/// However, be aware that just because percent-encoding plays no role in equality checking does not
/// mean that the query is normalized. If the query needs to be normalized, use the
/// [`Query::normalize`] function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Query<'query> {
    /// Whether the query is normalized.
    normalized: bool,

    /// The internal query source that is either owned or borrowed.
    query: Cow<'query, str>,
}

impl Query<'_> {
    /// Returns a new query which is identical but has a lifetime tied to this query.
    pub fn as_borrowed(&self) -> Query {
        use self::Cow::*;

        let query = match &self.query {
            Borrowed(borrowed) => *borrowed,
            Owned(owned) => owned.as_str(),
        };

        Query {
            normalized: self.normalized,
            query: Cow::Borrowed(query),
        }
    }

    /// Returns a `str` representation of the query.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Query;
    ///
    /// let query = Query::try_from("query").unwrap();
    /// assert_eq!(query.as_str(), "query");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.query
    }

    /// Converts the [`Query`] into an owned copy.
    ///
    /// If you construct the query from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the query will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Query<'static> {
        Query {
            normalized: self.normalized,
            query: Cow::from(self.query.into_owned()),
        }
    }

    /// Returns whether the query is normalized.
    ///
    /// A normalized query will have no bytes that are in the unreserved character set
    /// percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Query;
    ///
    /// let query = Query::try_from("query").unwrap();
    /// assert!(query.is_normalized());
    ///
    /// let mut query = Query::try_from("%ff%ff").unwrap();
    /// assert!(!query.is_normalized());
    /// query.normalize();
    /// assert!(query.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    /// Normalizes the query such that it will have no bytes that are in the unreserved character
    /// set percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// If the query is already normalized, the function will return immediately. Otherwise, if the
    /// query is not owned, this function will perform an allocation to clone it. The normalization
    /// itself though, is done in-place with no extra memory allocations required.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Query;
    ///
    /// let mut query = Query::try_from("query").unwrap();
    /// query.normalize();
    /// assert_eq!(query, "query");
    ///
    /// let mut query = Query::try_from("%ff%41").unwrap();
    /// assert_eq!(query, "%ff%41");
    /// query.normalize();
    /// assert_eq!(query, "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if !self.normalized {
            // Unsafe: Queries must be valid ASCII-US, so this is safe.
            unsafe { normalize_string(&mut self.query.to_mut(), true) };
            self.normalized = true;
        }
    }
}

impl AsRef<[u8]> for Query<'_> {
    fn as_ref(&self) -> &[u8] {
        self.query.as_bytes()
    }
}

impl AsRef<str> for Query<'_> {
    fn as_ref(&self) -> &str {
        &self.query
    }
}

impl Deref for Query<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.query
    }
}

impl Display for Query<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(&self.query)
    }
}

impl Eq for Query<'_> {}

impl<'query> From<Query<'query>> for String {
    fn from(value: Query<'query>) -> Self {
        value.to_string()
    }
}

impl Hash for Query<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        percent_encoded_hash(self.query.as_bytes(), state, true);
    }
}

impl PartialEq for Query<'_> {
    fn eq(&self, other: &Query) -> bool {
        *self == *other.as_bytes()
    }
}

impl PartialEq<[u8]> for Query<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        percent_encoded_equality(self.query.as_bytes(), other, true)
    }
}

impl<'query> PartialEq<Query<'query>> for [u8] {
    fn eq(&self, other: &Query<'query>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for Query<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'query> PartialEq<Query<'query>> for &'a [u8] {
    fn eq(&self, other: &Query<'query>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for Query<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'query> PartialEq<Query<'query>> for str {
    fn eq(&self, other: &Query<'query>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Query<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'query> PartialEq<Query<'query>> for &'a str {
    fn eq(&self, other: &Query<'query>) -> bool {
        other == self.as_bytes()
    }
}

impl<'query> TryFrom<&'query [u8]> for Query<'query> {
    type Error = QueryError;

    fn try_from(value: &'query [u8]) -> Result<Self, Self::Error> {
        let (query, rest) = parse_query(value)?;

        if rest.is_empty() {
            Ok(query)
        } else {
            Err(QueryError::InvalidCharacter)
        }
    }
}

impl<'query> TryFrom<&'query str> for Query<'query> {
    type Error = QueryError;

    fn try_from(value: &'query str) -> Result<Self, Self::Error> {
        Query::try_from(value.as_bytes())
    }
}

/// An error representing an invalid query.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum QueryError {
    /// The fragment contained an invalid character.
    InvalidCharacter,

    /// The fragment contained an invalid percent encoding (e.g. `"%ZZ"`).
    InvalidPercentEncoding,
}

impl Display for QueryError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::QueryError::*;

        match self {
            InvalidCharacter => write!(formatter, "invalid query character"),
            InvalidPercentEncoding => write!(formatter, "invalid query percent encoding"),
        }
    }
}

impl Error for QueryError {}

impl From<Infallible> for QueryError {
    fn from(_: Infallible) -> Self {
        QueryError::InvalidCharacter
    }
}

/// Parses the query from the given byte string.
pub(crate) fn parse_query(value: &[u8]) -> Result<(Query, &[u8]), QueryError> {
    let mut bytes = value.iter();
    let mut end_index = 0;
    let mut normalized = true;

    while let Some(&byte) = bytes.next() {
        match QUERY_CHAR_MAP[byte as usize] {
            0 if byte == b'#' => break,
            0 => return Err(QueryError::InvalidCharacter),
            b'%' => match get_percent_encoded_value(bytes.next().cloned(), bytes.next().cloned()) {
                Ok((hex_value, uppercase)) => {
                    if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                        normalized = false;
                    }

                    end_index += 3;
                }
                Err(_) => return Err(QueryError::InvalidPercentEncoding),
            },
            _ => end_index += 1,
        }
    }

    let (value, rest) = value.split_at(end_index);

    // Unsafe: The loop above makes sure the byte string is valid ASCII-US.
    let query = Query {
        normalized,
        query: Cow::from(unsafe { str::from_utf8_unchecked(value) }),
    };
    Ok((query, rest))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_query_normalize() {
        fn test_case(value: &str, expected: &str) {
            let mut query = Query::try_from(value).unwrap();
            query.normalize();
            assert_eq!(query, expected);
        }

        test_case("", "");
        test_case("%ff", "%FF");
        test_case("%41", "A");
    }

    #[test]
    fn test_query_parse() {
        use self::QueryError::*;

        assert_eq!(Query::try_from("").unwrap(), "");
        assert_eq!(Query::try_from("query").unwrap(), "query");
        assert_eq!(Query::try_from("qUeRy").unwrap(), "qUeRy");
        assert_eq!(Query::try_from("%ff%ff%ff%41").unwrap(), "%ff%ff%ff%41");

        assert_eq!(Query::try_from(" "), Err(InvalidCharacter));
        assert_eq!(Query::try_from("#"), Err(InvalidCharacter));
        assert_eq!(Query::try_from("%"), Err(InvalidPercentEncoding));
        assert_eq!(Query::try_from("%f"), Err(InvalidPercentEncoding));
        assert_eq!(Query::try_from("%zz"), Err(InvalidPercentEncoding));
    }
}
