#![warn(clippy::all)]

use percent_encoding::{percent_decode, utf8_percent_encode, AsciiSet, CONTROLS};
use std::iter::Iterator;

/// A query string. Holds a list of `(key,value)`.
///
/// Examples
///
/// Parameters can be get by their names.
///
/// ```
/// let qs = qstring::QString::from("?foo=bar%20baz");
/// let foo = qs.get("foo").unwrap();
/// assert_eq!(foo, "bar baz");
/// ```
///
/// Parameters not found are `None`.
///
/// ```
/// let qs = qstring::QString::from("?foo=bar");
/// let foo = &qs.get("panda");
/// assert!(foo.is_none());
/// ```
///
/// The query string can be assembled from pairs.
///
/// ```
/// let qs = qstring::QString::new(vec![
///    ("foo", "bar baz"),
///    ("panda", "true"),
/// ]);
/// assert_eq!(format!("{}", qs), "foo=bar%20baz&panda=true");
/// ```
///
#[derive(Clone, Debug, PartialEq, Default)]
pub struct QString {
    pairs: Vec<(String, QValue)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QValue {
    Empty,
    Value(String),
}

impl From<String> for QValue {
    fn from(s: String) -> QValue {
        QValue::Value(s)
    }
}

impl QString {
    /// Constructs a `QString` from a list of pairs.
    ///
    /// ```
    /// let qs = qstring::QString::new(vec![
    ///    ("foo", "bar baz"),
    ///    ("panda", "true"),
    /// ]);
    /// assert_eq!(format!("{}", qs), "foo=bar%20baz&panda=true");
    /// ```
    pub fn new<S, T>(params: Vec<(S, T)>) -> QString
    where
        S: Into<String>,
        T: Into<String>,
    {
        QString {
            pairs: params
                .into_iter()
                .map(|(k, v)| (k.into(), QValue::Value(v.into())))
                .collect(),
        }
    }

    /// Tells if a query parameter is present.
    ///
    /// ```
    /// let qs = qstring::QString::from("?foo");
    /// assert!(qs.has("foo"));
    /// assert!(qs.get("foo").is_some());
    /// ```
    pub fn has(&self, name: &str) -> bool {
        self.pairs.iter().any(|p| p.0 == name)
    }

    /// Get a query parameter by name.
    ///
    /// Empty query parameters (`?foo`) return `""`
    ///
    /// ```
    /// let qs = qstring::QString::from("?foo=bar");
    /// let foo = qs.get("foo");
    /// assert_eq!(foo, Some("bar"));
    /// ```
    pub fn get<'a>(&'a self, name: &str) -> Option<&'a str> {
        self.pairs
            .iter()
            .find(|p| p.0 == name)
            .and_then(|p| match p.1 {
                QValue::Empty => Some(""),
                QValue::Value(ref s) => Some(s),
            })
    }

    /// Converts the QString to list of pairs.
    ///
    /// ```
    /// let qs = qstring::QString::from("?foo=bar&baz=boo");
    /// let ps = qs.into_pairs();
    /// assert_eq!(ps, vec![
    ///     ("foo".to_string(), "bar".to_string()),
    ///     ("baz".to_string(), "boo".to_string()),
    /// ]);
    /// ```
    pub fn into_pairs(self) -> Vec<(String, String)> {
        self.pairs
            .into_iter()
            .map(|p| {
                (
                    p.0,
                    match p.1 {
                        QValue::Empty => "".to_string(),
                        QValue::Value(s) => s,
                    },
                )
            })
            .collect()
    }

    /// Represent the QString as a list of pairs.
    ///
    /// ```
    /// let qs = qstring::QString::from("?foo=bar&baz=boo");
    /// let ps = qs.to_pairs();
    /// assert_eq!(ps, vec![
    ///     ("foo", "bar"),
    ///     ("baz", "boo"),
    /// ]);
    /// ```
    pub fn to_pairs(&self) -> Vec<(&str, &str)> {
        self.pairs
            .iter()
            .map(|p| {
                (
                    p.0.as_str(),
                    match p.1 {
                        QValue::Empty => "",
                        QValue::Value(ref s) => s.as_str(),
                    },
                )
            })
            .collect()
    }

    /// Adds another query parameter pair.
    ///
    /// ```
    /// let mut qs = qstring::QString::from("?foo=bar&baz=boo");
    ///
    /// qs.add_pair(("panda", "bear"));
    ///
    /// assert_eq!(qs.to_string(), "foo=bar&baz=boo&panda=bear");
    /// ```
    pub fn add_pair<S, T>(&mut self, pair: (S, T))
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.pairs
            .push((pair.0.into(), QValue::Value(pair.1.into())));
    }

    /// Parse the string and add all found parameters to this instance.
    ///
    /// ```
    /// let mut qs = qstring::QString::from("?foo");
    ///
    /// qs.add_str("&bar=baz&pooch&panda=bear");
    ///
    /// assert_eq!(qs.to_string(), "foo&bar=baz&pooch&panda=bear");
    /// ```
    pub fn add_str(&mut self, origin: &str) {
        let mut to_add = str_to_pairs(origin);
        self.pairs.append(&mut to_add);
    }

    /// The number of query string pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// if this query string is empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

impl<'a> From<&'a str> for QString {
    /// Constructs a new `QString` by parsing a query string part of the URL.
    /// Can start with ? or not, either works.
    ///
    /// Examples
    ///
    /// ```
    /// let qs = qstring::QString::from("?foo=bar");
    /// let v: Vec<(String, String)> = qs.into_pairs();
    /// assert_eq!(v, vec![("foo".to_string(), "bar".to_string())]);
    /// ```
    fn from(origin: &str) -> Self {
        QString {
            pairs: str_to_pairs(origin),
        }
    }
}

fn str_to_pairs(origin: &str) -> Vec<(String, QValue)> {
    // current slice left to find params in
    let mut cur = origin;

    // move forward if start with ?
    if !cur.is_empty() && &cur[0..1] == "?" {
        cur = &cur[1..];
    }

    // where we build found parameters into
    let mut params = vec![];

    while !cur.is_empty() {
        // if we're positioned on a &, skip it
        if &cur[0..1] == "&" {
            cur = &cur[1..];
            continue;
        }
        // find position of next =
        let (name, rest) = match cur.find('=') {
            // no next =, name will be until next & or until end
            None => match cur.find('&') {
                // no &, name is until end
                None => {
                    params.push((decode(&cur[..]), QValue::Empty));
                    break;
                }
                // name is until next &, which means no value and shortcut
                // to start straight after the &.
                Some(pos) => {
                    params.push((decode(&cur[..pos]), QValue::Empty));
                    cur = &cur[(pos + 1)..];
                    continue;
                }
            },
            Some(pos) => {
                if let Some(apos) = cur.find('&') {
                    if apos < pos {
                        params.push((decode(&cur[..apos]), QValue::Empty));
                        cur = &cur[(apos + 1)..];
                        continue;
                    }
                }
                (&cur[..pos], &cur[(pos + 1)..])
            }
        };
        // skip parameters with no name
        if name.is_empty() {
            cur = rest;
            continue;
        }
        // from rest, find next occurence of &
        let (value, newcur) = match rest.find('&') {
            // no next &, then value is all up until end
            None => (rest, ""),
            // found one, value is up until & and next round starts after.
            Some(pos) => (&rest[..pos], &rest[(pos + 1)..]),
        };
        // found a parameter
        params.push((decode(name), QValue::Value(decode(value))));
        cur = newcur;
    }
    params
}

impl IntoIterator for QString {
    type Item = (String, String);
    type IntoIter = ::std::vec::IntoIter<(String, String)>;
    fn into_iter(self) -> Self::IntoIter {
        self.into_pairs().into_iter()
    }
}

impl Into<Vec<(String, String)>> for QString {
    fn into(self) -> Vec<(String, String)> {
        self.into_pairs()
    }
}

impl Into<String> for QString {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl ::std::fmt::Display for QString {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        for (idx, p) in self.pairs.iter().enumerate() {
            write!(
                f,
                "{}{}{}",
                (if idx == 0 { "" } else { "&" }),
                encode(&p.0),
                match p.1 {
                    QValue::Empty => "".to_string(),
                    QValue::Value(ref s) => format!("={}", encode(s)),
                }
            )?;
        }
        Ok(())
    }
}

fn decode(s: &str) -> String {
    percent_decode(s.as_bytes())
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .unwrap_or_else(|_| s.to_string())
}

const FRAGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'&')
    .add(b'?')
    .add(b'=');

fn encode(s: &str) -> String {
    utf8_percent_encode(s, FRAGMENT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test {
        ($func_name:ident, $origin:expr, $result:expr) => {
            #[test]
            fn $func_name() {
                let qs = QString::from($origin);
                let ps: Vec<(String, String)> = qs.into_pairs();
                let cs: Vec<(String, String)> = ($result as Vec<(&str, &str)>)
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                assert_eq!(ps, cs);
            }
        };
    }

    #[test]
    fn encode_amp() {
        let x = QString::new(vec![("foo", "b&?=ar")]);
        assert_eq!("foo=b%26%3F%3Dar", x.to_string());
    }

    #[test]
    fn amps_in_a_row() {
        assert_eq!(
            QString::from("&bar=baz&pooch&panda=bear").to_pairs(),
            vec![("bar", "baz"), ("pooch", ""), ("panda", "bear")]
        );
    }

    test!(empty_1, "", vec![]);
    test!(empty_2, "?", vec![]);
    test!(empty_3, "&", vec![]);
    test!(empty_4, "=", vec![]);
    test!(empty_5, "?=", vec![]);
    test!(empty_6, "?&", vec![]);

    test!(a_is_1, "a", vec![("a", "")]);
    test!(a_is_2, "a=", vec![("a", "")]);
    test!(a_is_3, "a=b", vec![("a", "b")]);
    test!(a_is_4, "?a", vec![("a", "")]);
    test!(a_is_5, "?a=", vec![("a", "")]);
    test!(a_is_6, "?a=b", vec![("a", "b")]);
    test!(a_is_7, "?&a", vec![("a", "")]);
    test!(a_is_8, "?&a=", vec![("a", "")]);
    test!(a_is_9, "?&a=b", vec![("a", "b")]);
    test!(a_is_10, "?a=&", vec![("a", "")]);
    test!(a_is_11, "?=a", vec![("a", "")]);

    test!(a_is_eq_1, "a==", vec![("a", "=")]);

    test!(is_q_1, "??", vec![("?", "")]);
    test!(is_q_2, "&?", vec![("?", "")]);
    test!(is_q_3, "??a", vec![("?a", "")]);
    test!(is_q_4, "&?a", vec![("?a", "")]);

    test!(ac_is_1, "?a&c", vec![("a", ""), ("c", "")]);
    test!(ac_is_2, "?a&c&", vec![("a", ""), ("c", "")]);
    test!(ac_is_3, "?a=&c", vec![("a", ""), ("c", "")]);
    test!(ac_is_4, "?a=&c=", vec![("a", ""), ("c", "")]);
    test!(ac_is_5, "?a=b&c=", vec![("a", "b"), ("c", "")]);
    test!(ac_is_6, "?a=&c=d", vec![("a", ""), ("c", "d")]);
    test!(ac_is_7, "?a=b&c=d", vec![("a", "b"), ("c", "d")]);
}
