//! Path Component
//!
//! See [[RFC3986, Section 3.3](https://tools.ietf.org/html/rfc3986#section-3.3)].

use std::borrow::Cow;
use std::convert::{Infallible, TryFrom};
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str;

use crate::utility::{
    get_percent_encoded_value, normalize_string, percent_encoded_equality, percent_encoded_hash,
    UNRESERVED_CHAR_MAP,
};

/// A map of byte characters that determines if a character is a valid path character.
#[rustfmt::skip]
const PATH_CHAR_MAP: [u8; 256] = [
 // 0     1     2     3     4     5     6     7     8     9     A     B     C     D     E     F
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 0
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 1
    0, b'!',    0,    0, b'$', b'%', b'&',b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.',    0, // 2
 b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b':', b';',    0, b'=',    0,    0, // 3
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

/// The path component as defined in
/// [[RFC3986, Section 3.3](https://tools.ietf.org/html/rfc3986#section-3.3)].
///
/// A path is composed of a sequence of segments. It is also either absolute or relative, where an
/// absolute path starts with a `'/'`. A URI with an authority *always* has an absolute path
/// regardless of whether the path was empty (i.e. "http://example.com" has a single empty
/// path segment and is absolute).
///
/// Each segment in the path is case-sensitive. Furthermore, percent-encoding plays no role in
/// equality checking for characters in the unreserved character set meaning that `"segment"` and
/// `"s%65gment"` are identical. Both of these attributes are reflected in the equality and hash
/// functions.
///
/// However, be aware that just because percent-encoding plays no role in equality checking does not
/// mean that either the path or a given segment is normalized. If the path or a segment needs to be
/// normalized, use either the [`Path::normalize`] or [`Segment::normalize`] functions,
/// respectively.
#[derive(Clone, Debug, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Path<'path> {
    /// whether the path is absolute. Specifically, a path is absolute if it starts with a
    /// `'/'`.
    absolute: bool,

    /// The total number of double dot segments in the path.
    double_dot_segment_count: u16,

    /// The number of double dot segments consecutive from the beginning of the path.
    leading_double_dot_segment_count: u16,

    /// The sequence of segments that compose the path.
    segments: Vec<Segment<'path>>,

    /// The total number of single dot segments in the path.
    single_dot_segment_count: u16,

    /// The total number of unnormalized segments in the path.
    unnormalized_count: u16,
}

impl<'path> Path<'path> {
    /// Clears all segments from the path leaving a single empty segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let mut path = Path::try_from("/my/path").unwrap();
    /// assert_eq!(path, "/my/path");
    /// path.clear();
    /// assert_eq!(path, "/");
    /// ```
    pub fn clear(&mut self) {
        self.segments.clear();
        self.segments.push(Segment::empty());
    }

    /// Converts the [`Path`] into an owned copy.
    ///
    /// If you construct the path from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the path will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Path<'static> {
        let segments = self
            .segments
            .into_iter()
            .map(Segment::into_owned)
            .collect::<Vec<Segment<'static>>>();

        Path {
            absolute: self.absolute,
            double_dot_segment_count: self.double_dot_segment_count,
            leading_double_dot_segment_count: self.leading_double_dot_segment_count,
            segments,
            single_dot_segment_count: self.single_dot_segment_count,
            unnormalized_count: self.unnormalized_count,
        }
    }

    /// Returns whether the path is absolute (i.e. it starts with a `'/'`).
    ///
    /// Any path following an [`Authority`] will *always* be parsed to be absolute.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let path = Path::try_from("/my/path").unwrap();
    /// assert_eq!(path.is_absolute(), true);
    /// ```
    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// Returns whether the path is normalized either as or as not a reference.
    ///
    /// See [`Path::normalize`] for a full description of what path normalization entails.
    ///
    /// Although this function does not operate in constant-time in general, it will be
    /// constant-time in the vast majority of cases.
    ///
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let path = Path::try_from("/my/path").unwrap();
    /// assert!(path.is_normalized(false));
    ///
    /// let path = Path::try_from("/my/p%61th").unwrap();
    /// assert!(!path.is_normalized(false));
    ///
    /// let path = Path::try_from("..").unwrap();
    /// assert!(path.is_normalized(true));
    ///
    /// let path = Path::try_from("../.././.").unwrap();
    /// assert!(!path.is_normalized(true));
    /// ```
    pub fn is_normalized(&self, as_reference: bool) -> bool {
        if self.unnormalized_count != 0 {
            return false;
        }

        if self.absolute || !as_reference {
            self.single_dot_segment_count == 0 && self.double_dot_segment_count == 0
        } else {
            (self.single_dot_segment_count == 0
                || (self.single_dot_segment_count == 1
                    && self.segments[0].is_single_dot_segment()
                    && self.segments.len() > 1
                    && self.segments[1].contains(':')))
                && self.double_dot_segment_count == self.leading_double_dot_segment_count
        }
    }

    /// Returns whether the path is relative (i.e. it does not start with a `'/'`).
    ///
    /// Any path following an [`Authority`] will *always* be parsed to be absolute.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let path = Path::try_from("my/path").unwrap();
    /// assert_eq!(path.is_relative(), true);
    /// ```
    pub fn is_relative(&self) -> bool {
        !self.absolute
    }

    /// Creates a path with no segments on it.
    ///
    /// This is only used to avoid allocations for temporary paths. Any path created using this
    /// function is **not** valid!
    pub(crate) unsafe fn new_with_no_segments(absolute: bool) -> Path<'static> {
        Path {
            absolute,
            double_dot_segment_count: 0,
            leading_double_dot_segment_count: 0,
            segments: Vec::new(),
            single_dot_segment_count: 0,
            unnormalized_count: 0,
        }
    }

    /// Normalizes the path and all of its segments.
    ///
    /// There are two components to path normalization, the normalization of each segment
    /// individually and the removal of unnecessary dot segments. It is also guaranteed that whether
    /// the path is absolute will not change as a result of normalization.
    ///
    /// The normalization of each segment will proceed according to [`Segment::normalize`].
    ///
    /// If the path is absolute (i.e., it starts with a `'/'`), then `as_reference` will be set to
    /// `false` regardless of its set value.
    ///
    /// If `as_reference` is `false`, then all dot segments will be removed as they would be if you
    /// had called [`Path::remove_dot_segments`]. Otherwise, when a dot segment is removed is
    /// dependent on whether it's `"."` or `".."` and its location in the path.
    ///
    /// In general, `"."` dot segments are always removed except for when it is at the beginning of
    /// the path and is followed by a segment containing a `':'`, e.g. `"./a:b"` stays the same.
    ///
    /// For `".."` dot segments, they are kept whenever they are at the beginning of the path and
    /// removed whenever they are not, e.g. `"a/../.."` normalizes to `".."`.
    pub fn normalize(&mut self, as_reference: bool) {
        if self.is_normalized(as_reference) {
            return;
        }

        self.unnormalized_count = 0;

        if self.absolute || !as_reference {
            self.remove_dot_segments_helper(true);
            return;
        }

        let mut double_dot_segment_count = 0;
        let mut last_dot_segment = None;
        let mut new_length = 0;

        for i in 0..self.segments.len() {
            let segment = &self.segments[i];

            if segment.is_single_dot_segment()
                && (new_length > 0
                    || i == self.segments.len() - 1
                    || !self.segments[i + 1].as_str().contains(':'))
            {
                continue;
            }

            if segment.is_double_dot_segment() {
                match last_dot_segment {
                    None if new_length == 0 => (),
                    Some(index) if index == new_length - 1 => (),
                    _ => {
                        if new_length == 2
                            && self.segments[0].is_single_dot_segment()
                            && (i == self.segments.len() - 1
                                || !self.segments[i + 1].as_str().contains(':'))
                        {
                            new_length -= 1
                        }

                        new_length -= 1;

                        continue;
                    }
                }

                double_dot_segment_count += 1;
                last_dot_segment = Some(new_length);
            }

            self.segments.swap(i, new_length);
            self.segments[new_length].normalize();
            new_length += 1;
        }

        if new_length == 0 {
            self.segments[0] = Segment::empty();
            new_length = 1;
        }

        self.double_dot_segment_count = double_dot_segment_count;
        self.leading_double_dot_segment_count = double_dot_segment_count;
        self.single_dot_segment_count = if self.segments[0].is_single_dot_segment() {
            1
        } else {
            0
        };

        self.segments.truncate(new_length);
    }

    /// Pops the last segment off of the path.
    ///
    /// If the path only contains one segment, then that segment will become empty.
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let mut path = Path::try_from("/my/path").unwrap();
    /// path.pop();
    /// assert_eq!(path, "/my");
    /// path.pop();
    /// assert_eq!(path, "/");
    /// ```
    pub fn pop(&mut self) {
        let segment = self.segments.pop().unwrap();

        if segment.is_single_dot_segment() {
            self.single_dot_segment_count =
                self.single_dot_segment_count.checked_sub(1).unwrap_or(0);
        }

        if segment.is_double_dot_segment() {
            self.double_dot_segment_count =
                self.double_dot_segment_count.checked_sub(1).unwrap_or(0);

            if self.double_dot_segment_count < self.leading_double_dot_segment_count {
                self.leading_double_dot_segment_count -= 1;
            }
        }

        if !segment.is_normalized() {
            self.unnormalized_count = self.unnormalized_count.checked_sub(1).unwrap_or(0);
        }

        if self.segments.is_empty() {
            self.segments.push(Segment::empty());
        }
    }

    /// Pushes a segment onto the path.
    ///
    /// If the conversion to a [`Segment`] fails, an [`InvalidPath`] will be returned.
    ///
    /// The behavior of this function is different if the current path is just one empty segment. In
    /// this case, the pushed segment will replace that empty segment unless the pushed segment is
    /// itself empty.
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let mut path = Path::try_from("/my/path").unwrap();
    /// path.push("test");
    /// assert_eq!(path, "/my/path/test");
    ///
    /// let mut path = Path::try_from("/").unwrap();
    /// path.push("test");
    /// assert_eq!(path, "/test");
    ///
    /// let mut path = Path::try_from("/").unwrap();
    /// path.push("");
    /// assert_eq!(path, "//");
    /// ```
    pub fn push<TSegment, TSegmentError>(&mut self, segment: TSegment) -> Result<(), PathError>
    where
        Segment<'path>: TryFrom<TSegment, Error = TSegmentError>,
        PathError: From<TSegmentError>,
    {
        if self.segments.len() as u16 == u16::max_value() {
            return Err(PathError::ExceededMaximumLength);
        }

        let segment = Segment::try_from(segment)?;

        if segment.is_single_dot_segment() {
            self.single_dot_segment_count += 1;
        }

        if segment.is_double_dot_segment() {
            if self.segments.len() as u16 == self.double_dot_segment_count {
                self.leading_double_dot_segment_count += 1;
            }

            self.double_dot_segment_count += 1;
        }

        if !segment.is_normalized() {
            self.unnormalized_count += 1;
        }

        if segment != "" && self.segments.len() == 1 && self.segments[0].as_str().is_empty() {
            self.segments[0] = segment;
        } else {
            self.segments.push(segment);
        }

        Ok(())
    }

    /// Removes all dot segments from the path according to the algorithm described in
    /// [[RFC3986, Section 5.2.4](https://tools.ietf.org/html/rfc3986#section-5.2.4)].
    ///
    /// This function will perform no memory allocations during removal of dot segments.
    ///
    /// If the path currently has no dot segments, then this function is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let mut path = Path::try_from("/a/b/c/./../../g").unwrap();
    /// path.remove_dot_segments();
    /// assert_eq!(path, "/a/g");
    /// ```
    pub fn remove_dot_segments(&mut self) {
        if self.single_dot_segment_count == 0 && self.double_dot_segment_count == 0 {
            return;
        }

        self.remove_dot_segments_helper(false);
    }

    /// Helper function that removes all dot segments with optional segment normalization.
    fn remove_dot_segments_helper(&mut self, normalize_segments: bool) {
        let mut input_absolute = self.absolute;
        let mut new_length = 0;

        for i in 0..self.segments.len() {
            let segment = &self.segments[i];

            if input_absolute {
                if segment.is_single_dot_segment() {
                    continue;
                } else if segment.is_double_dot_segment() {
                    if new_length > 0 {
                        new_length -= 1;
                    } else {
                        self.absolute = false;
                    }

                    continue;
                }

                if new_length == 0 {
                    self.absolute = true;
                }
            } else if segment.is_single_dot_segment() || segment.is_double_dot_segment() {
                continue;
            }

            self.segments.swap(i, new_length);

            if normalize_segments {
                self.segments[new_length].normalize();
            }

            new_length += 1;

            if i < self.segments.len() - 1 {
                input_absolute = true;
            } else {
                input_absolute = false;
            }
        }

        if input_absolute {
            if new_length == 0 {
                self.absolute = true;
            } else {
                self.segments[new_length] = Segment::empty();
                new_length += 1;
            }
        }

        if new_length == 0 {
            self.segments[0] = Segment::empty();
            new_length = 1;
        }

        self.double_dot_segment_count = 0;
        self.leading_double_dot_segment_count = 0;
        self.single_dot_segment_count = 0;
        self.segments.truncate(new_length);
    }

    /// Returns the segments of the path.
    ///
    /// If you require mutability, use [`Path::segments_mut`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let mut path = Path::try_from("/my/path").unwrap();
    /// assert_eq!(path.segments()[1], "path");
    /// ```
    pub fn segments(&self) -> &[Segment<'path>] {
        &self.segments
    }

    /// Returns the segments of the path mutably.
    ///
    /// Due to the required restriction that there must be at least one segment in a path, this
    /// mutability only applies to the segments themselves, not the container.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Path, Segment};
    ///
    /// let mut path = Path::try_from("/my/path").unwrap();
    /// let mut segments = path.segments_mut();
    /// segments[1] = Segment::try_from("test").unwrap();
    ///
    /// assert_eq!(path, "/my/test");
    /// ```
    pub fn segments_mut(&mut self) -> &mut [Segment<'path>] {
        &mut self.segments
    }

    /// Sets whether the path is absolute (i.e. it starts with a `'/'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Path;
    ///
    /// let mut path = Path::try_from("/my/path").unwrap();
    /// path.set_absolute(false);
    /// assert_eq!(path, "my/path");
    /// ```
    pub fn set_absolute(&mut self, absolute: bool) {
        self.absolute = absolute;
    }

    /// Returns a new path which is identical but has a lifetime tied to this path.
    ///
    /// This function will perform a memory allocation.
    pub fn to_borrowed(&self) -> Path {
        let segments = self.segments.iter().map(Segment::as_borrowed).collect();

        Path {
            absolute: self.absolute,
            double_dot_segment_count: self.double_dot_segment_count,
            leading_double_dot_segment_count: self.leading_double_dot_segment_count,
            segments,
            single_dot_segment_count: self.single_dot_segment_count,
            unnormalized_count: self.unnormalized_count,
        }
    }
}

impl Display for Path<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if self.absolute {
            formatter.write_char('/')?;
        }

        for (index, segment) in self.segments.iter().enumerate() {
            formatter.write_str(segment.as_str())?;

            if index < self.segments.len() - 1 {
                formatter.write_char('/')?;
            }
        }

        Ok(())
    }
}

impl Hash for Path<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.segments.hash(state)
    }
}

impl<'path> From<Path<'path>> for String {
    fn from(value: Path<'path>) -> Self {
        value.to_string()
    }
}

impl PartialEq for Path<'_> {
    fn eq(&self, other: &Path) -> bool {
        self.segments == other.segments
    }
}

impl PartialEq<[u8]> for Path<'_> {
    fn eq(&self, mut other: &[u8]) -> bool {
        if self.absolute {
            match other.get(0) {
                Some(&byte) => {
                    if byte != b'/' {
                        return false;
                    }
                }
                None => return false,
            }

            other = &other[1..];
        }

        for (index, segment) in self.segments.iter().enumerate() {
            let len = segment.as_str().len();

            if other.len() < len || &other[..len] != segment {
                return false;
            }

            other = &other[len..];

            if index < self.segments.len() - 1 {
                match other.get(0) {
                    Some(&byte) => {
                        if byte != b'/' {
                            return false;
                        }
                    }
                    None => return false,
                }

                other = &other[1..];
            }
        }

        true
    }
}

impl<'path> PartialEq<Path<'path>> for [u8] {
    fn eq(&self, other: &Path<'path>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for Path<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'path> PartialEq<Path<'path>> for &'a [u8] {
    fn eq(&self, other: &Path<'path>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for Path<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'path> PartialEq<Path<'path>> for str {
    fn eq(&self, other: &Path<'path>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Path<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'path> PartialEq<Path<'path>> for &'a str {
    fn eq(&self, other: &Path<'path>) -> bool {
        other == self.as_bytes()
    }
}

impl<'path> TryFrom<&'path [u8]> for Path<'path> {
    type Error = PathError;

    fn try_from(value: &'path [u8]) -> Result<Self, Self::Error> {
        let (path, rest) = parse_path(value)?;

        if rest.is_empty() {
            Ok(path)
        } else {
            Err(PathError::InvalidCharacter)
        }
    }
}

impl<'path> TryFrom<&'path str> for Path<'path> {
    type Error = PathError;

    fn try_from(value: &'path str) -> Result<Self, Self::Error> {
        Path::try_from(value.as_bytes())
    }
}

/// A segment of a path.
///
/// Segments are separated from other segments with the `'/'` delimiter.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Segment<'segment> {
    /// Whether the segment is normalized.
    normalized: bool,

    /// The internal segment source that is either owned or borrowed.
    segment: Cow<'segment, str>,
}

impl Segment<'_> {
    /// Returns a new segment which is identical but has as lifetime tied to this segment.
    pub fn as_borrowed(&self) -> Segment {
        use self::Cow::*;

        let segment = match &self.segment {
            Borrowed(borrowed) => *borrowed,
            Owned(owned) => owned.as_str(),
        };

        Segment {
            normalized: self.normalized,
            segment: Cow::Borrowed(segment),
        }
    }

    /// Returns a `str` representation of the segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Segment;
    ///
    /// let segment = Segment::try_from("segment").unwrap();
    /// assert_eq!(segment.as_str(), "segment");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.segment
    }

    /// Constructs a segment that is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use uriparse::Segment;
    ///
    /// assert_eq!(Segment::empty(),  "");
    /// ```
    pub fn empty() -> Segment<'static> {
        Segment {
            normalized: true,
            segment: Cow::from(""),
        }
    }

    /// Converts the [`Segment`] into an owned copy.
    ///
    /// If you construct the segment from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the segment will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Segment<'static> {
        Segment {
            normalized: self.normalized,
            segment: Cow::from(self.segment.into_owned()),
        }
    }

    /// Returns whether the segment is a dot segment, i.e., is `"."` or `".."`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Segment;
    ///
    /// let segment = Segment::try_from("segment").unwrap();
    /// assert!(!segment.is_dot_segment());
    ///
    /// let segment = Segment::try_from(".").unwrap();
    /// assert!(segment.is_dot_segment());
    ///
    /// let segment = Segment::try_from("..").unwrap();
    /// assert!(segment.is_dot_segment());
    /// ```
    pub fn is_dot_segment(&self) -> bool {
        self == "." || self == ".."
    }

    /// Returns whether the segment is a dot segment, i.e., is `".."`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Segment;
    ///
    /// let segment = Segment::try_from("segment").unwrap();
    /// assert!(!segment.is_double_dot_segment());
    ///
    /// let segment = Segment::try_from(".").unwrap();
    /// assert!(!segment.is_double_dot_segment());
    ///
    /// let segment = Segment::try_from("..").unwrap();
    /// assert!(segment.is_double_dot_segment());
    /// ```
    pub fn is_double_dot_segment(&self) -> bool {
        self == ".."
    }

    /// Returns whether the segment is normalized.
    ///
    /// A normalized segment will have no bytes that are in the unreserved character set
    /// percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Segment;
    ///
    /// let segment = Segment::try_from("segment").unwrap();
    /// assert!(segment.is_normalized());
    ///
    /// let mut segment = Segment::try_from("%ff%ff").unwrap();
    /// assert!(!segment.is_normalized());
    /// segment.normalize();
    /// assert!(segment.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    /// Returns whether the segment is a dot segment, i.e., is `"."`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Segment;
    ///
    /// let segment = Segment::try_from("segment").unwrap();
    /// assert!(!segment.is_single_dot_segment());
    ///
    /// let segment = Segment::try_from(".").unwrap();
    /// assert!(segment.is_single_dot_segment());
    ///
    /// let segment = Segment::try_from("..").unwrap();
    /// assert!(!segment.is_single_dot_segment());
    /// ```
    pub fn is_single_dot_segment(&self) -> bool {
        self == "."
    }

    /// Normalizes the segment such that it will have no bytes that are in the unreserved character
    /// set percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// If the segment is already normalized, the function will return immediately. Otherwise, if
    /// the segment is not owned, this function will perform an allocation to clone it. The
    /// normalization itself though, is done in-place with no extra memory allocations required.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Segment;
    ///
    /// let mut segment = Segment::try_from("segment").unwrap();
    /// segment.normalize();
    /// assert_eq!(segment, "segment");
    ///
    /// let mut segment = Segment::try_from("%ff%41").unwrap();
    /// assert_eq!(segment, "%ff%41");
    /// segment.normalize();
    /// assert_eq!(segment, "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if !self.normalized {
            // Unsafe: Paths must be valid ASCII-US, so this is safe.
            unsafe { normalize_string(&mut self.segment.to_mut(), true) };
            self.normalized = true;
        }
    }
}

impl AsRef<[u8]> for Segment<'_> {
    fn as_ref(&self) -> &[u8] {
        self.segment.as_bytes()
    }
}

impl AsRef<str> for Segment<'_> {
    fn as_ref(&self) -> &str {
        &self.segment
    }
}

impl Deref for Segment<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.segment
    }
}

impl Display for Segment<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(&self.segment)
    }
}

impl Eq for Segment<'_> {}

impl<'segment> From<Segment<'segment>> for String {
    fn from(value: Segment<'segment>) -> Self {
        value.to_string()
    }
}

impl Hash for Segment<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        percent_encoded_hash(self.segment.as_bytes(), state, true);
    }
}

impl PartialEq for Segment<'_> {
    fn eq(&self, other: &Segment) -> bool {
        *self == *other.as_bytes()
    }
}

impl PartialEq<[u8]> for Segment<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        percent_encoded_equality(self.segment.as_bytes(), other, true)
    }
}

impl<'segment> PartialEq<Segment<'segment>> for [u8] {
    fn eq(&self, other: &Segment<'segment>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for Segment<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'segment> PartialEq<Segment<'segment>> for &'a [u8] {
    fn eq(&self, other: &Segment<'segment>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for Segment<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'segment> PartialEq<Segment<'segment>> for str {
    fn eq(&self, other: &Segment<'segment>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Segment<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'segment> PartialEq<Segment<'segment>> for &'a str {
    fn eq(&self, other: &Segment<'segment>) -> bool {
        other == self.as_bytes()
    }
}

impl<'segment> TryFrom<&'segment [u8]> for Segment<'segment> {
    type Error = PathError;

    fn try_from(value: &'segment [u8]) -> Result<Self, Self::Error> {
        let mut bytes = value.iter();
        let mut normalized = true;

        while let Some(&byte) = bytes.next() {
            match PATH_CHAR_MAP[byte as usize] {
                0 => return Err(PathError::InvalidCharacter),
                b'%' => {
                    match get_percent_encoded_value(bytes.next().cloned(), bytes.next().cloned()) {
                        Ok((hex_value, uppercase)) => {
                            if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                                normalized = false;
                            }
                        }
                        Err(_) => return Err(PathError::InvalidPercentEncoding),
                    }
                }
                _ => (),
            }
        }

        // Unsafe: The loop above makes sure the byte string is valid ASCII-US.
        let segment = Segment {
            normalized,
            segment: Cow::Borrowed(unsafe { str::from_utf8_unchecked(value) }),
        };
        Ok(segment)
    }
}

impl<'segment> TryFrom<&'segment str> for Segment<'segment> {
    type Error = PathError;

    fn try_from(value: &'segment str) -> Result<Self, Self::Error> {
        Segment::try_from(value.as_bytes())
    }
}

/// An error representing an invalid path.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PathError {
    /// The path exceeded the maximum length allowed. Due to implementation reasons, the maximum
    /// length a path can be is 2^16 or 65536 characters.
    ExceededMaximumLength,

    /// The path contained an invalid character.
    InvalidCharacter,

    /// The path contained an invalid percent encoding (e.g. `"%ZZ"`).
    InvalidPercentEncoding,
}

impl Display for PathError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::PathError::*;

        match self {
            ExceededMaximumLength => write!(formatter, "exceeded maximum path length"),
            InvalidCharacter => write!(formatter, "invalid path character"),
            InvalidPercentEncoding => write!(formatter, "invalid path percent encoding"),
        }
    }
}

impl Error for PathError {}

impl From<Infallible> for PathError {
    fn from(_: Infallible) -> Self {
        PathError::InvalidCharacter
    }
}

/// Parses the path from the given byte string.
pub(crate) fn parse_path(value: &[u8]) -> Result<(Path, &[u8]), PathError> {
    struct SegmentInfo {
        absolute: bool,
        double_dot_segment_count: u16,
        index: u16,
        last_double_dot_segment: Option<u16>,
        leading_double_dot_segment_count: u16,
        normalized: bool,
        single_dot_segment_count: u16,
        unnormalized_count: u16,
    }

    impl SegmentInfo {
        fn into_path<'path>(self, segments: Vec<Segment<'path>>) -> Path<'path> {
            Path {
                absolute: self.absolute,
                double_dot_segment_count: self.double_dot_segment_count,
                leading_double_dot_segment_count: self.leading_double_dot_segment_count,
                segments,
                single_dot_segment_count: self.single_dot_segment_count,
                unnormalized_count: self.unnormalized_count,
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn new_segment<'segment>(
        segment: &'segment [u8],
        segment_info: &mut SegmentInfo,
    ) -> Segment<'segment> {
        if !segment_info.normalized {
            segment_info.unnormalized_count += 1;
        }

        if segment == b"." {
            segment_info.single_dot_segment_count += 1;
        }

        if segment == b".." {
            let index = segment_info.index - 1;
            segment_info.double_dot_segment_count += 1;

            if index == 0 || segment_info.last_double_dot_segment == Some(index - 1) {
                segment_info.leading_double_dot_segment_count += 1;
                segment_info.last_double_dot_segment = Some(index);
            }
        }

        // Unsafe: The loop above makes sure the byte string is valid ASCII-US.
        Segment {
            normalized: segment_info.normalized,
            segment: Cow::from(unsafe { str::from_utf8_unchecked(segment) }),
        }
    }

    let (value, absolute) = if value.starts_with(b"/") {
        (&value[1..], true)
    } else {
        (value, false)
    };

    let mut bytes = value.iter();
    let mut segment_info = SegmentInfo {
        absolute,
        double_dot_segment_count: 0,
        index: 1,
        last_double_dot_segment: None,
        leading_double_dot_segment_count: 0,
        normalized: true,
        single_dot_segment_count: 0,
        unnormalized_count: 0,
    };
    let mut segment_end_index = 0;
    let mut segment_start_index = 0;

    // Set some moderate initial capacity. This seems to help with performance a bit.
    let mut segments = Vec::with_capacity(10);

    while let Some(&byte) = bytes.next() {
        match PATH_CHAR_MAP[byte as usize] {
            0 if byte == b'?' || byte == b'#' => {
                let segment = new_segment(
                    &value[segment_start_index..segment_end_index],
                    &mut segment_info,
                );
                segments.push(segment);
                let path = segment_info.into_path(segments);
                return Ok((path, &value[segment_end_index..]));
            }
            0 if byte == b'/' => {
                let segment = new_segment(
                    &value[segment_start_index..segment_end_index],
                    &mut segment_info,
                );
                segments.push(segment);
                segment_end_index += 1;
                segment_start_index = segment_end_index;
                segment_info.index = segment_info
                    .index
                    .checked_add(1)
                    .ok_or(PathError::ExceededMaximumLength)?;
                segment_info.normalized = true;
            }
            0 => return Err(PathError::InvalidCharacter),
            b'%' => match get_percent_encoded_value(bytes.next().cloned(), bytes.next().cloned()) {
                Ok((hex_value, uppercase)) => {
                    if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                        segment_info.normalized = false;
                    }

                    segment_end_index += 3;
                }
                Err(_) => return Err(PathError::InvalidPercentEncoding),
            },
            _ => segment_end_index += 1,
        }
    }

    let segment = new_segment(&value[segment_start_index..], &mut segment_info);
    segments.push(segment);
    let path = segment_info.into_path(segments);
    Ok((path, b""))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_path_equals() {
        assert_eq!(
            Path::try_from("segment").unwrap(),
            Path::try_from("s%65gment").unwrap()
        );
    }

    #[test]
    fn test_path_normalize() {
        fn test_case(value: &str, expected: &str, as_reference: bool) {
            let mut path = Path::try_from(value).unwrap();
            path.normalize(as_reference);

            let expected_single_dot_segment_count = if expected.starts_with("./") { 1 } else { 0 };
            let expected_double_dot_segment_count = expected
                .split('/')
                .filter(|&segment| segment == "..")
                .count() as u16;

            assert!(!path.segments().is_empty());
            assert!(path.is_normalized(as_reference));
            assert_eq!(
                path.single_dot_segment_count,
                expected_single_dot_segment_count
            );
            assert_eq!(
                path.double_dot_segment_count,
                expected_double_dot_segment_count
            );
            assert_eq!(
                path.leading_double_dot_segment_count,
                expected_double_dot_segment_count
            );
            assert_eq!(path.to_string(), expected);
        }

        test_case("", "", true);
        test_case(".", "", true);
        test_case("..", "..", true);
        test_case("../", "../", true);
        test_case("/.", "/", true);
        test_case("./././././././.", "", true);
        test_case("././././././././", "", true);
        test_case("/..", "/", true);
        test_case("../..", "../..", true);
        test_case("../a/../..", "../..", true);
        test_case("a", "a", true);
        test_case("a/..", "", true);
        test_case("a/../", "", true);
        test_case("a/../..", "..", true);
        test_case("./a:b", "./a:b", true);
        test_case("./a:b/..", "", true);
        test_case("./a:b/../c:d", "./c:d", true);
        test_case("./../a:b", "../a:b", true);
        test_case("../a/../", "../", true);
        test_case("../../.././.././../../../.", "../../../../../../..", true);
        test_case("a/.././a:b", "./a:b", true);

        test_case("", "", false);
        test_case(".", "", false);
        test_case("..", "", false);
        test_case("../", "", false);
        test_case("/.", "/", false);
        test_case("/..", "/", false);
        test_case("../../.././.././../../../.", "", false);
        test_case("a/../..", "/", false);
        test_case("a/../../", "/", false);
        test_case("/a/../../../../", "/", false);
        test_case("/a/./././././././c", "/a/c", false);
        test_case("/a/.", "/a/", false);
        test_case("/a/./", "/a/", false);
        test_case("/a/..", "/", false);
        test_case("/a/b/./..", "/a/", false);
        test_case("/a/b/./../", "/a/", false);
        test_case("/a/b/c/./../../g", "/a/g", false);
        test_case("mid/content=5/../6", "mid/6", false);

        test_case("this/is/a/t%65st/path/%ff", "this/is/a/test/path/%FF", true);
        test_case(
            "this/is/a/t%65st/path/%ff",
            "this/is/a/test/path/%FF",
            false,
        );
    }

    #[test]
    fn test_path_parse() {
        use self::PathError::*;

        let slash = "/".to_string();

        assert_eq!(Path::try_from("").unwrap(), "");
        assert_eq!(Path::try_from("/").unwrap(), "/");
        assert_eq!(
            Path::try_from("/tHiS/iS/a/PaTh").unwrap(),
            "/tHiS/iS/a/PaTh"
        );
        assert_eq!(Path::try_from("%ff%ff%ff%41").unwrap(), "%ff%ff%ff%41");
        assert!(Path::try_from(&*slash.repeat(65535)).is_ok());

        assert_eq!(
            Path::try_from(&*slash.repeat(65536)),
            Err(ExceededMaximumLength)
        );
        assert_eq!(Path::try_from(" "), Err(InvalidCharacter));
        assert_eq!(Path::try_from("#"), Err(InvalidCharacter));
        assert_eq!(Path::try_from("%"), Err(InvalidPercentEncoding));
        assert_eq!(Path::try_from("%f"), Err(InvalidPercentEncoding));
        assert_eq!(Path::try_from("%zz"), Err(InvalidPercentEncoding));
    }

    #[test]
    fn test_path_remove_dot_segments() {
        fn test_case(value: &str, expected: &str) {
            let mut path = Path::try_from(value).unwrap();
            path.remove_dot_segments();
            assert!(!path.segments().is_empty());
            assert_eq!(path.single_dot_segment_count, 0);
            assert_eq!(path.double_dot_segment_count, 0);
            assert_eq!(path.leading_double_dot_segment_count, 0);
            assert_eq!(path.to_string(), expected);
        }

        test_case("", "");
        test_case(".", "");
        test_case("..", "");
        test_case("../", "");
        test_case("/.", "/");
        test_case("/..", "/");
        test_case("../../.././.././../../../.", "");
        test_case("a/../..", "/");
        test_case("a/../../", "/");
        test_case("/a/../../../..", "/");
        test_case("/a/../../../../", "/");
        test_case("/a/./././././././c", "/a/c");
        test_case("/a/.", "/a/");
        test_case("/a/./", "/a/");
        test_case("/a/..", "/");
        test_case("/a/b/./..", "/a/");
        test_case("/a/b/./../", "/a/");
        test_case("/a/b/c/./../../g", "/a/g");
        test_case("mid/content=5/../6", "mid/6");
    }

    #[test]
    fn test_segment_normalize() {
        fn test_case(value: &str, expected: &str) {
            let mut segment = Segment::try_from(value).unwrap();
            segment.normalize();
            assert_eq!(segment, expected);
        }

        test_case("", "");
        test_case("%ff", "%FF");
        test_case("%41", "A");
    }

    #[test]
    fn test_segment_parse() {
        use self::PathError::*;

        assert_eq!(Segment::try_from("").unwrap(), "");
        assert_eq!(Segment::try_from("segment").unwrap(), "segment");
        assert_eq!(Segment::try_from("sEgMeNt").unwrap(), "sEgMeNt");
        assert_eq!(Segment::try_from("%ff%ff%ff%41").unwrap(), "%ff%ff%ff%41");

        assert_eq!(Segment::try_from(" "), Err(InvalidCharacter));
        assert_eq!(Segment::try_from("/"), Err(InvalidCharacter));
        assert_eq!(Segment::try_from("%"), Err(InvalidPercentEncoding));
        assert_eq!(Segment::try_from("%f"), Err(InvalidPercentEncoding));
        assert_eq!(Segment::try_from("%zz"), Err(InvalidPercentEncoding));
    }
}
