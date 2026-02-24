use std::convert::{Infallible, TryFrom};
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};
use std::mem;

use crate::authority::{parse_authority, Authority, AuthorityError, Host, Password, Username};
use crate::fragment::{Fragment, FragmentError};
use crate::path::{parse_path, Path, PathError};
use crate::query::{parse_query, Query, QueryError};
use crate::scheme::{parse_scheme, Scheme, SchemeError};

/// A URI reference as defined in
/// [[RFC3986, Section 4.1]](https://tools.ietf.org/html/rfc3986#section-4.1).
///
/// Specifically, a URI reference is either a URI or a relative reference (a schemeless URI).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct URIReference<'uri> {
    /// The authority component of the URI reference as defined in
    /// [[RFC3986, Section 3.2]](https://tools.ietf.org/html/rfc3986#section-3.2).
    authority: Option<Authority<'uri>>,

    /// The fragment component of the URI reference as defined in
    /// [[RFC3986, Section 3.5]](https://tools.ietf.org/html/rfc3986#section-3.5).
    fragment: Option<Fragment<'uri>>,

    /// The path component of the URI reference as defined in
    /// [[RFC3986, Section 3.3]](https://tools.ietf.org/html/rfc3986#section-3.3).
    path: Path<'uri>,

    /// The query component of the URI reference as defined in
    /// [[RFC3986, Section 3.4]](https://tools.ietf.org/html/rfc3986#section-3.4).
    query: Option<Query<'uri>>,

    /// The scheme component of the URI reference as defined in
    /// [[RFC3986, Section 3.1](https://tools.ietf.org/html/rfc3986#section-3.1).
    scheme: Option<Scheme<'uri>>,
}

impl<'uri> URIReference<'uri> {
    /// Returns the authority, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("//example.com/my/path").unwrap();
    /// assert_eq!(reference.authority().unwrap().to_string(), "example.com");
    /// ```
    pub fn authority(&self) -> Option<&Authority<'uri>> {
        self.authority.as_ref()
    }

    /// Constructs a default builder for a URI reference.
    ///
    /// This provides an alternative means of constructing a URI reference besides parsing and
    /// [`URIReference::from_parts`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Path, Scheme, URIReference};
    ///
    /// let reference = URIReference::builder()
    ///     .with_scheme(Some(Scheme::HTTP))
    ///     .with_authority(Some(Authority::try_from("example.com").unwrap()))
    ///     .with_path(Path::try_from("/my/path").unwrap())
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(reference.to_string(), "http://example.com/my/path");
    /// ```
    pub fn builder<'new_uri>() -> URIReferenceBuilder<'new_uri> {
        URIReferenceBuilder::default()
    }

    /// Returns whether the URI reference can act as a base URI.
    ///
    /// A URI can be a base if it is absolute (i.e. it has no fragment component).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com/my/path").unwrap();
    /// assert!(reference.can_be_a_base());
    ///
    /// let reference = URIReference::try_from("ftp://127.0.0.1#fragment").unwrap();
    /// assert!(!reference.can_be_a_base());
    /// ```
    pub fn can_be_a_base(&self) -> bool {
        self.has_scheme() && !self.has_fragment()
    }

    /// Constructs a new [`URIReference`] from the individual parts: scheme, authority, path, query,
    /// and fragment.
    ///
    /// The lifetime used by the resulting value will be the lifetime of the part that is most
    /// restricted in scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Scheme, URIReference};
    ///
    /// let reference = URIReference::from_parts(
    ///     None::<Scheme>,
    ///     Some("example.com"),
    ///     "/my/path",
    ///     Some("query"),
    ///     Some("fragment")
    /// ).unwrap();
    /// assert_eq!(reference.to_string(), "//example.com/my/path?query#fragment");
    /// ```
    pub fn from_parts<
        'new_uri,
        TScheme,
        TAuthority,
        TPath,
        TQuery,
        TFragment,
        TSchemeError,
        TAuthorityError,
        TPathError,
        TQueryError,
        TFragmentError,
    >(
        scheme: Option<TScheme>,
        authority: Option<TAuthority>,
        path: TPath,
        query: Option<TQuery>,
        fragment: Option<TFragment>,
    ) -> Result<URIReference<'new_uri>, URIReferenceError>
    where
        Scheme<'new_uri>: TryFrom<TScheme, Error = TSchemeError>,
        Authority<'new_uri>: TryFrom<TAuthority, Error = TAuthorityError>,
        Path<'new_uri>: TryFrom<TPath, Error = TPathError>,
        Query<'new_uri>: TryFrom<TQuery, Error = TQueryError>,
        Fragment<'new_uri>: TryFrom<TFragment, Error = TFragmentError>,
        URIReferenceError: From<TSchemeError>
            + From<TAuthorityError>
            + From<TPathError>
            + From<TQueryError>
            + From<TFragmentError>,
    {
        let scheme = match scheme {
            Some(scheme) => Some(Scheme::try_from(scheme)?),
            None => None,
        };

        let authority = match authority {
            Some(authority) => Some(Authority::try_from(authority)?),
            None => None,
        };

        let mut path = Path::try_from(path)?;

        if authority.is_some() {
            path.set_absolute(true);
        }

        validate_absolute_path(authority.as_ref(), &path)?;
        validate_schemeless_path(scheme.as_ref(), authority.as_ref(), &path)?;

        let query = match query {
            Some(query) => Some(Query::try_from(query)?),
            None => None,
        };

        let fragment = match fragment {
            Some(fragment) => Some(Fragment::try_from(fragment)?),
            None => None,
        };

        Ok(URIReference {
            authority,
            fragment,
            path,
            query,
            scheme,
        })
    }

    /// Returns the fragment, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com#fragment").unwrap();
    /// assert_eq!(reference.fragment().unwrap(), "fragment");
    /// ```
    pub fn fragment(&self) -> Option<&Fragment<'uri>> {
        self.fragment.as_ref()
    }

    /// Returns whether the URI reference has an authority component.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com").unwrap();
    /// assert!(reference.has_authority());
    ///
    /// let reference = URIReference::try_from("").unwrap();
    /// assert!(!reference.has_authority());
    /// ```
    pub fn has_authority(&self) -> bool {
        self.authority.is_some()
    }

    /// Returns whether the URI reference has a fragment component.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("#test").unwrap();
    /// assert!(reference.has_fragment());
    ///
    /// let reference = URIReference::try_from("http://example.com").unwrap();
    /// assert!(!reference.has_fragment());
    /// ```
    pub fn has_fragment(&self) -> bool {
        self.fragment.is_some()
    }

    /// Returns whether the URI reference has a password component.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://user:pass@127.0.0.1").unwrap();
    /// assert!(reference.has_password());
    ///
    /// let reference = URIReference::try_from("http://user@127.0.0.1").unwrap();
    /// assert!(!reference.has_password());
    /// ```
    pub fn has_password(&self) -> bool {
        if let Some(ref authority) = self.authority {
            authority.has_password()
        } else {
            false
        }
    }

    /// Returns whether the URI reference has a port.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://127.0.0.1:8080").unwrap();
    /// assert!(reference.has_port());
    ///
    /// let reference = URIReference::try_from("http://127.0.0.1").unwrap();
    /// assert!(!reference.has_port());
    /// ```
    pub fn has_port(&self) -> bool {
        if let Some(ref authority) = self.authority {
            authority.has_port()
        } else {
            false
        }
    }

    /// Returns whether the URI reference has a query component.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("/?my=query").unwrap();
    /// assert!(reference.has_query());
    ///
    /// let reference = URIReference::try_from("http://example.com/my/path").unwrap();
    /// assert!(!reference.has_query());
    /// ```
    pub fn has_query(&self) -> bool {
        self.query.is_some()
    }

    /// Returns whether the URI reference has a scheme component.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com?my=query").unwrap();
    /// assert!(reference.has_scheme());
    ///
    /// let reference = URIReference::try_from("/my/path").unwrap();
    /// assert!(!reference.has_scheme());
    /// ```
    pub fn has_scheme(&self) -> bool {
        self.scheme.is_some()
    }

    /// Returns whether the URI reference has a username component.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("//username@example.com").unwrap();
    /// assert!(reference.has_username());
    ///
    /// let reference = URIReference::try_from("http://example.com").unwrap();
    /// assert!(!reference.has_username());
    /// ```
    pub fn has_username(&self) -> bool {
        if let Some(ref authority) = self.authority {
            authority.has_username()
        } else {
            false
        }
    }

    /// Returns the host, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://username@example.com").unwrap();
    /// assert_eq!(reference.host().unwrap().to_string(), "example.com");
    /// ```
    pub fn host(&self) -> Option<&Host<'uri>> {
        if let Some(ref authority) = self.authority {
            Some(authority.host())
        } else {
            None
        }
    }

    /// Consumes the URI reference and converts it into a builder with the same values.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Fragment, Query, URIReference};
    ///
    /// let reference = URIReference::try_from("//example.com/path?query#fragment").unwrap();
    /// let mut builder = reference.into_builder();
    /// builder.query(None::<Query>).fragment(None::<Fragment>);
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "//example.com/path");
    /// ```
    pub fn into_builder(self) -> URIReferenceBuilder<'uri> {
        let mut builder = URIReferenceBuilder::new();
        builder
            .scheme(self.scheme)
            .authority(self.authority)
            .path(self.path)
            .query(self.query)
            .fragment(self.fragment);
        builder
    }

    /// Converts the [`URIReference`] into an owned copy.
    ///
    /// If you construct the URI reference from a source with a non-static lifetime, you may run
    /// into lifetime problems due to the way the struct is designed. Calling this function will
    /// ensure that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the URI reference will just copy the
    /// references, and thus the lifetime will remain the same.
    pub fn into_owned(self) -> URIReference<'static> {
        let scheme = self.scheme.map(Scheme::into_owned);
        let authority = self.authority.map(Authority::into_owned);
        let path = self.path.into_owned();
        let query = self.query.map(Query::into_owned);
        let fragment = self.fragment.map(Fragment::into_owned);

        URIReference {
            authority,
            fragment,
            path,
            query,
            scheme,
        }
    }

    /// Consumes the [`URIReference`] and returns its parts: scheme, authority, path, query, and
    /// fragment.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from(
    ///     "http://username:password@example.com:80/my/path?my=query#fragment",
    /// ).unwrap();
    /// let (scheme, authority, path, query, fragment) = reference.into_parts();
    ///
    /// assert_eq!(scheme.unwrap(), "http");
    /// assert_eq!(authority.unwrap().to_string(), "username:password@example.com:80");
    /// assert_eq!(path, "/my/path");
    /// assert_eq!(query.unwrap(), "my=query");
    /// assert_eq!(fragment.unwrap(), "fragment");
    /// ```
    pub fn into_parts(
        self,
    ) -> (
        Option<Scheme<'uri>>,
        Option<Authority<'uri>>,
        Path<'uri>,
        Option<Query<'uri>>,
        Option<Fragment<'uri>>,
    ) {
        (
            self.scheme,
            self.authority,
            self.path,
            self.query,
            self.fragment,
        )
    }

    /// Returns whether the URI reference is an absolute path reference.
    ///
    /// A URI reference is an absolute path reference if it is a relative reference that begins with
    /// a single `'/'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("/my/path").unwrap();
    /// assert!(reference.is_absolute_path_reference());
    /// ```
    pub fn is_absolute_path_reference(&self) -> bool {
        self.scheme.is_none() && self.authority.is_none() && self.path.is_absolute()
    }

    /// Returns whether the URI reference is a network path reference.
    ///
    /// A URI reference is a network path reference if it is a relative reference that begins with
    /// two `'/'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("//example.com").unwrap();
    /// assert!(reference.is_network_path_reference());
    /// ```
    pub fn is_network_path_reference(&self) -> bool {
        self.scheme.is_none() && self.authority.is_some()
    }

    /// Returns whether the URI reference is normalized.
    ///
    /// A normalized URI reference will have all of its components normalized.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com/?a=b").unwrap();
    /// assert!(reference.is_normalized());
    ///
    /// let mut reference = URIReference::try_from("http://EXAMPLE.com/?a=b").unwrap();
    /// assert!(!reference.is_normalized());
    /// reference.normalize();
    /// assert!(reference.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        if let Some(scheme) = self.scheme.as_ref() {
            if !scheme.is_normalized() {
                return false;
            }
        }

        if let Some(authority) = self.authority.as_ref() {
            if !authority.is_normalized() {
                return false;
            }
        }

        if !self.path.is_normalized(self.scheme.is_none()) {
            return false;
        }

        if let Some(query) = self.query.as_ref() {
            if !query.is_normalized() {
                return false;
            }
        }

        if let Some(fragment) = self.fragment.as_ref() {
            if !fragment.is_normalized() {
                return false;
            }
        }

        true
    }

    /// Returns whether the URI reference is a relative path reference.
    ///
    /// A URI reference is a relative path reference if it is a relative reference that does not
    /// begin with a `'/'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("my/path").unwrap();
    /// assert!(reference.is_relative_path_reference());
    /// ```
    pub fn is_relative_path_reference(&self) -> bool {
        self.scheme.is_none() && self.authority.is_none() && !self.path.is_absolute()
    }

    /// Returns whether the URI reference is a relative reference.
    ///
    /// A URI reference is a relative reference if it has no scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("/my/path").unwrap();
    /// assert!(reference.is_relative_reference());
    /// ```
    pub fn is_relative_reference(&self) -> bool {
        self.scheme.is_none()
    }

    /// Returns whether the URI reference is a URI.
    ///
    /// A URI reference is a URI if it has a scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com").unwrap();
    /// assert!(reference.is_uri());
    /// ```
    pub fn is_uri(&self) -> bool {
        self.scheme.is_some()
    }

    /// Maps the authority using the given map function.
    ///
    /// This function will panic if, as a result of the authority change, the URI reference becomes
    /// invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, URIReference};
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.map_authority(|_| Some(Authority::try_from("127.0.0.1").unwrap()));
    /// assert_eq!(reference.to_string(), "http://127.0.0.1/");
    /// ```
    pub fn map_authority<TMapper>(&mut self, mapper: TMapper) -> Option<&Authority<'uri>>
    where
        TMapper: FnOnce(Option<Authority<'uri>>) -> Option<Authority<'uri>>,
    {
        let authority = mapper(self.authority.take());
        self.set_authority(authority)
            .expect("mapped authority resulted in invalid state")
    }

    /// Maps the fragment using the given map function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Fragment, URIReference};
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.map_fragment(|_| Some(Fragment::try_from("fragment").unwrap()));
    /// assert_eq!(reference.to_string(), "http://example.com/#fragment");
    /// ```
    pub fn map_fragment<TMapper>(&mut self, mapper: TMapper) -> Option<&Fragment<'uri>>
    where
        TMapper: FnOnce(Option<Fragment<'uri>>) -> Option<Fragment<'uri>>,
    {
        let fragment = mapper(self.fragment.take());
        self.set_fragment(fragment)
            .expect("mapped fragment resulted in invalid state")
    }

    /// Maps the path using the given map function.
    ///
    /// This function will panic if, as a result of the path change, the URI reference becomes
    /// invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, URIReference};
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.map_path(|mut path| {
    ///     path.push("test").unwrap();
    ///     path.push("path").unwrap();
    ///     path
    /// });
    /// assert_eq!(reference.to_string(), "http://example.com/test/path");
    /// ```
    pub fn map_path<TMapper>(&mut self, mapper: TMapper) -> &Path<'uri>
    where
        TMapper: FnOnce(Path<'uri>) -> Path<'uri>,
    {
        // Unsafe: We're creating an invalid path just as a temporary sentinel value, but it is
        // replaced shortly after.
        let temp_path = unsafe { Path::new_with_no_segments(true) };
        let path = mapper(mem::replace(&mut self.path, temp_path));
        self.set_path(path)
            .expect("mapped path resulted in invalid state")
    }

    /// Maps the query using the given map function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Query, URIReference};
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.map_query(|_| Some(Query::try_from("query").unwrap()));
    /// assert_eq!(reference.to_string(), "http://example.com/?query");
    /// ```
    pub fn map_query<TMapper>(&mut self, mapper: TMapper) -> Option<&Query<'uri>>
    where
        TMapper: FnOnce(Option<Query<'uri>>) -> Option<Query<'uri>>,
    {
        let query = mapper(self.query.take());
        self.set_query(query)
            .expect("mapped query resulted in invalid state")
    }

    /// Maps the scheme using the given map function.
    ///
    /// This function will panic if, as a result of the scheme change, the URI reference becomes
    /// invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Scheme, URIReference};
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.map_scheme(|_| Some(Scheme::try_from("https").unwrap()));
    /// assert_eq!(reference.to_string(), "https://example.com/");
    /// ```
    pub fn map_scheme<TMapper>(&mut self, mapper: TMapper) -> Option<&Scheme<'uri>>
    where
        TMapper: FnOnce(Option<Scheme<'uri>>) -> Option<Scheme<'uri>>,
    {
        let scheme = mapper(self.scheme.take());
        self.set_scheme(scheme)
            .expect("mapped scheme resulted in invalid state")
    }

    /// Normalizes the URI reference.
    ///
    /// A normalized URI reference will have all of its components normalized.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let mut reference = URIReference::try_from("http://example.com/?a=b").unwrap();
    /// reference.normalize();
    /// assert_eq!(reference.to_string(), "http://example.com/?a=b");
    ///
    /// let mut reference = URIReference::try_from("http://EXAMPLE.com/?a=b").unwrap();
    /// assert_eq!(reference.to_string(), "http://EXAMPLE.com/?a=b");
    /// reference.normalize();
    /// assert_eq!(reference.to_string(), "http://example.com/?a=b");
    /// ```
    pub fn normalize(&mut self) {
        if let Some(scheme) = self.scheme.as_mut() {
            scheme.normalize();
        }

        if let Some(authority) = self.authority.as_mut() {
            authority.normalize();
        }

        self.path.normalize(self.scheme.is_none());

        if let Some(query) = self.query.as_mut() {
            query.normalize();
        }

        if let Some(fragment) = self.fragment.as_mut() {
            fragment.normalize();
        }
    }

    /// Returns the path of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://127.0.0.1/my/path").unwrap();
    /// assert_eq!(reference.path(), "/my/path");
    /// ```
    pub fn path(&self) -> &Path<'uri> {
        &self.path
    }

    /// Returns the password, if present, of the URI reference.
    ///
    /// Usage of a password in URI and URI references is deprecated.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://user:pass@example.com").unwrap();
    /// assert_eq!(reference.password().unwrap(), "pass");
    /// ```
    pub fn password(&self) -> Option<&Password<'uri>> {
        if let Some(ref authority) = self.authority {
            authority.password()
        } else {
            None
        }
    }

    /// Returns the port, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://example.com:8080/").unwrap();
    /// assert_eq!(reference.port().unwrap(), 8080);
    /// ```
    pub fn port(&self) -> Option<u16> {
        if let Some(ref authority) = self.authority {
            authority.port()
        } else {
            None
        }
    }

    /// Returns the query, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://127.0.0.1?my=query").unwrap();
    /// assert_eq!(reference.query().unwrap(), "my=query");
    /// ```
    pub fn query(&self) -> Option<&Query<'uri>> {
        self.query.as_ref()
    }

    /// Returns the scheme, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://127.0.0.1/").unwrap();
    /// assert_eq!(reference.scheme().unwrap(), "http");
    /// ```
    pub fn scheme(&self) -> Option<&Scheme<'uri>> {
        self.scheme.as_ref()
    }

    /// Sets the authority of the URI reference.
    ///
    /// An error will be returned if the conversion to an [`Authority`] fails.
    ///
    /// The existing path will be set to absolute (i.e. starts with a `'/'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.set_authority(Some("user@example.com:80"));
    /// assert_eq!(reference.to_string(), "http://user@example.com:80/");
    /// ```
    pub fn set_authority<TAuthority, TAuthorityError>(
        &mut self,
        authority: Option<TAuthority>,
    ) -> Result<Option<&Authority<'uri>>, URIReferenceError>
    where
        Authority<'uri>: TryFrom<TAuthority, Error = TAuthorityError>,
        URIReferenceError: From<TAuthorityError>,
    {
        self.authority = match authority {
            Some(authority) => {
                self.path.set_absolute(true);
                Some(Authority::try_from(authority)?)
            }
            None => {
                validate_absolute_path(None, &self.path)?;
                None
            }
        };
        Ok(self.authority())
    }

    /// Sets the fragment of the URI reference.
    ///
    /// An error will be returned if the conversion to a [`Fragment`] fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.set_fragment(Some("fragment"));
    /// assert_eq!(reference.to_string(), "http://example.com/#fragment");
    /// ```
    pub fn set_fragment<TFragment, TFragmentError>(
        &mut self,
        fragment: Option<TFragment>,
    ) -> Result<Option<&Fragment<'uri>>, URIReferenceError>
    where
        Fragment<'uri>: TryFrom<TFragment, Error = TFragmentError>,
        URIReferenceError: From<TFragmentError>,
    {
        self.fragment = match fragment {
            Some(fragment) => Some(Fragment::try_from(fragment)?),
            None => None,
        };
        Ok(self.fragment())
    }

    /// Sets the path of the URI reference.
    ///
    /// An error will be returned in one of two cases:
    ///  - The conversion to [`Path`] failed.
    ///  - The path was set to a value that resulted in an invalid URI reference.
    ///
    /// Regardless of whether the given path was set as absolute or relative, if the URI
    /// reference currently has an authority, the path will be forced to be absolute.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.set_path("my/path");
    /// assert_eq!(reference.to_string(), "http://example.com/my/path");
    /// ```
    pub fn set_path<TPath, TPathError>(
        &mut self,
        path: TPath,
    ) -> Result<&Path<'uri>, URIReferenceError>
    where
        Path<'uri>: TryFrom<TPath, Error = TPathError>,
        URIReferenceError: From<TPathError>,
    {
        let mut path = Path::try_from(path)?;
        validate_absolute_path(self.authority.as_ref(), &path)?;
        validate_schemeless_path(self.scheme.as_ref(), self.authority.as_ref(), &path)?;

        if self.authority.is_some() {
            path.set_absolute(true);
        }

        self.path = path;
        Ok(self.path())
    }

    /// Sets the query of the URI reference.
    ///
    /// An error will be returned if the conversion to a [`Query`] fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.set_query(Some("myquery"));
    /// assert_eq!(reference.to_string(), "http://example.com/?myquery");
    /// ```
    pub fn set_query<TQuery, TQueryError>(
        &mut self,
        query: Option<TQuery>,
    ) -> Result<Option<&Query<'uri>>, URIReferenceError>
    where
        Query<'uri>: TryFrom<TQuery, Error = TQueryError>,
        URIReferenceError: From<TQueryError>,
    {
        self.query = match query {
            Some(query) => Some(Query::try_from(query)?),
            None => None,
        };
        Ok(self.query())
    }

    /// Sets the scheme of the URI reference.
    ///
    /// An error will be returned in one of two cases:
    ///  - The conversion to [`Scheme`] failed.
    ///  - The scheme was set to `None`, but the resulting URI reference has an invalid schemeless
    ///    path.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let mut reference = URIReference::try_from("http://example.com").unwrap();
    /// reference.set_scheme(Some("https"));
    /// assert_eq!(reference.to_string(), "https://example.com/");
    /// ```
    pub fn set_scheme<TScheme, TSchemeError>(
        &mut self,
        scheme: Option<TScheme>,
    ) -> Result<Option<&Scheme<'uri>>, URIReferenceError>
    where
        Scheme<'uri>: TryFrom<TScheme, Error = TSchemeError>,
        URIReferenceError: From<TSchemeError>,
    {
        self.scheme = match scheme {
            Some(scheme) => Some(Scheme::try_from(scheme)?),
            None => {
                validate_schemeless_path(None, self.authority.as_ref(), &self.path)?;
                None
            }
        };
        Ok(self.scheme())
    }

    /// Returns a new URI reference which is identical but has a lifetime tied to this URI
    /// reference.
    ///
    /// This function will perform a memory allocation.
    pub fn to_borrowed(&self) -> URIReference {
        let scheme = self.scheme.as_ref().map(Scheme::as_borrowed);
        let authority = self.authority.as_ref().map(Authority::as_borrowed);
        let path = self.path.to_borrowed();
        let query = self.query.as_ref().map(Query::as_borrowed);
        let fragment = self.fragment.as_ref().map(Fragment::as_borrowed);

        URIReference {
            authority,
            fragment,
            path,
            query,
            scheme,
        }
    }

    /// Returns the username, if present, of the URI reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::URIReference;
    ///
    /// let reference = URIReference::try_from("http://username@example.com").unwrap();
    /// assert_eq!(reference.username().unwrap(), "username");
    /// ```
    pub fn username(&self) -> Option<&Username<'uri>> {
        if let Some(ref authority) = self.authority {
            authority.username()
        } else {
            None
        }
    }
}

impl Display for URIReference<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if let Some(ref scheme) = self.scheme {
            formatter.write_str(scheme.as_str())?;
            formatter.write_char(':')?;
        }

        if let Some(ref authority) = self.authority {
            formatter.write_str("//")?;
            formatter.write_str(&authority.to_string())?;
        }

        formatter.write_str(&self.path.to_string())?;

        if let Some(ref query) = self.query {
            formatter.write_char('?')?;
            formatter.write_str(query.as_str())?;
        }

        if let Some(ref fragment) = self.fragment {
            formatter.write_char('#')?;
            formatter.write_str(fragment.as_str())?;
        }

        Ok(())
    }
}

impl<'uri> From<URIReference<'uri>> for String {
    fn from(value: URIReference<'uri>) -> Self {
        value.to_string()
    }
}

impl<'uri> TryFrom<&'uri [u8]> for URIReference<'uri> {
    type Error = URIReferenceError;

    fn try_from(value: &'uri [u8]) -> Result<Self, Self::Error> {
        let (scheme, value) = match parse_scheme(value) {
            Ok((scheme, rest)) => {
                if rest.starts_with(b":") {
                    (Some(scheme), &rest[1..])
                } else {
                    (None, value)
                }
            }
            _ => (None, value),
        };

        let (authority, value) = match value.get(0..2) {
            Some(b"//") => {
                let (authority, value) = parse_authority(&value[2..])?;
                (Some(authority), value)
            }
            _ => (None, value),
        };

        let (mut path, value) = parse_path(value)?;

        if authority.is_some() {
            path.set_absolute(true);
        }

        validate_schemeless_path(scheme.as_ref(), authority.as_ref(), &path)?;

        let (query, value) = if value.starts_with(b"?") {
            let (query, value) = parse_query(&value[1..])?;
            (Some(query), value)
        } else {
            (None, value)
        };

        let fragment = if value.starts_with(b"#") {
            Some(Fragment::try_from(&value[1..])?)
        } else {
            None
        };

        Ok(URIReference {
            authority,
            fragment,
            path,
            query,
            scheme,
        })
    }
}

impl<'uri> TryFrom<&'uri str> for URIReference<'uri> {
    type Error = URIReferenceError;

    fn try_from(value: &'uri str) -> Result<Self, Self::Error> {
        URIReference::try_from(value.as_bytes())
    }
}

/// A builder type for [`URIReference]`.
///
/// You must use the [`URIReference::path`] function before building as URI references always have
/// have a path. Everything else is optional.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct URIReferenceBuilder<'uri> {
    /// The authority component of the URI reference as defined in
    /// [[RFC3986, Section 3.2]](https://tools.ietf.org/html/rfc3986#section-3.2).
    authority: Option<Authority<'uri>>,

    /// The fragment component of the URI reference as defined in
    /// [[RFC3986, Section 3.5]](https://tools.ietf.org/html/rfc3986#section-3.5).
    fragment: Option<Fragment<'uri>>,

    /// The path component of the URI reference as defined in
    /// [[RFC3986, Section 3.3]](https://tools.ietf.org/html/rfc3986#section-3.3).
    path: Option<Path<'uri>>,

    /// The query component of the URI reference as defined in
    /// [[RFC3986, Section 3.4]](https://tools.ietf.org/html/rfc3986#section-3.4).
    query: Option<Query<'uri>>,

    /// The scheme component of the URI reference as defined in
    /// [[RFC3986, Section 3.1]](https://tools.ietf.org/html/rfc3986#section-3.1).
    scheme: Option<Scheme<'uri>>,
}

impl<'uri> URIReferenceBuilder<'uri> {
    /// Sets the authority part of the URI reference.
    ///
    /// It is optional to specify a authority.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Path, URIReferenceBuilder};
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .authority(Some(Authority::try_from("example.com").unwrap()))
    ///     .path(Path::try_from("/my/path").unwrap());
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "//example.com/my/path");
    /// ```
    pub fn authority(&mut self, authority: Option<Authority<'uri>>) -> &mut Self {
        self.authority = authority;
        self
    }

    /// Consumes the builder and tries to build a [`URIReference`].
    ///
    /// This function will error in one of two situations:
    ///  - A path was not specified in the builder.
    ///  - While all individual components were valid, their combination as a URI reference was
    ///    invalid.
    ///
    /// # Examples
    ///
    /// First error type (path not specified):
    ///
    /// ```
    /// use uriparse::URIReferenceBuilder;
    ///
    /// let result = URIReferenceBuilder::new().build();
    /// assert!(result.is_err());
    /// ```
    ///
    /// Second error type (first segment in schemeless path cannot contain a `':'`):
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Path, URIReferenceBuilder};
    ///
    /// let result = URIReferenceBuilder::new()
    ///     .with_path(Path::try_from("my:/path").unwrap())
    ///     .build();
    /// assert!(result.is_err());
    /// ```
    pub fn build(self) -> Result<URIReference<'uri>, URIReferenceError> {
        let path = match self.path {
            Some(path) => path,
            None => return Err(URIReferenceError::MissingPath),
        };

        URIReference::from_parts(self.scheme, self.authority, path, self.query, self.fragment)
    }

    /// Sets the fragment part of the URI reference.
    ///
    /// It is optional to specify a fragment.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Fragment, Path, URIReferenceBuilder};
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .path(Path::try_from("/my/path").unwrap())
    ///     .fragment(Some(Fragment::try_from("fragment").unwrap()));
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "/my/path#fragment");
    /// ```
    pub fn fragment(&mut self, fragment: Option<Fragment<'uri>>) -> &mut Self {
        self.fragment = fragment;
        self
    }

    /// Constructs a new builder with nothing set.
    pub fn new() -> Self {
        URIReferenceBuilder::default()
    }

    /// Sets the path part of the URI reference.
    ///
    /// It is required to specify a path. Not doing so will result in an error during the
    /// [`URIReferenceBuilder::build`] function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Path, URIReferenceBuilder};
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .path(Path::try_from("/my/path").unwrap());
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "/my/path");
    /// ```
    pub fn path(&mut self, path: Path<'uri>) -> &mut Self {
        self.path = Some(path);
        self
    }

    /// Sets the query part of the URI reference.
    ///
    /// It is optional to specify a query.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Path, Query, URIReferenceBuilder};
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .path(Path::try_from("/my/path").unwrap())
    ///     .query(Some(Query::try_from("query").unwrap()));
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "/my/path?query");
    /// ```
    pub fn query(&mut self, query: Option<Query<'uri>>) -> &mut Self {
        self.query = query;
        self
    }

    /// Sets the scheme part of the URI reference.
    ///
    /// It is optional to specify a scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Path, Scheme, URIReferenceBuilder};
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .scheme(Some(Scheme::HTTP))
    ///     .authority(Some(Authority::try_from("example.com").unwrap()))
    ///     .path(Path::try_from("/my/path").unwrap());
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "http://example.com/my/path");
    /// ```
    pub fn scheme(&mut self, scheme: Option<Scheme<'uri>>) -> &mut Self {
        self.scheme = scheme;
        self
    }

    /// Sets the authority part of the URI reference.
    ///
    /// If the given authority is not a valid authority (i.e. the conversion fails), an error is
    /// return.
    ///
    /// It is optional to specify an authority.
    ///
    /// # Examples
    ///
    /// ```
    /// use uriparse::URIReferenceBuilder;
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .try_authority(Some("example.com"))
    ///     .unwrap()
    ///     .try_path("/my/path")
    ///     .unwrap();
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "//example.com/my/path");
    /// ```
    pub fn try_authority<TAuthority, TAuthorityError>(
        &mut self,
        authority: Option<TAuthority>,
    ) -> Result<&mut Self, TAuthorityError>
    where
        Authority<'uri>: TryFrom<TAuthority, Error = TAuthorityError>,
        AuthorityError: From<TAuthorityError>,
    {
        self.authority = match authority {
            Some(authority) => Some(Authority::try_from(authority).map_err(|error| error)?),
            None => None,
        };
        Ok(self)
    }

    /// Sets the fragment part of the URI reference.
    ///
    /// If the given fragment is not a valid fragment (i.e. the conversion fails), an error is
    /// returned.
    ///
    /// It is optional to specify a fragment.
    ///
    /// # Examples
    ///
    /// ```
    /// use uriparse::URIReferenceBuilder;
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .try_path("/my/path")
    ///     .unwrap()
    ///     .try_fragment(Some("fragment"))
    ///     .unwrap();
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "/my/path#fragment");
    /// ```
    pub fn try_fragment<TFragment, TFragmentError>(
        &mut self,
        fragment: Option<TFragment>,
    ) -> Result<&mut Self, FragmentError>
    where
        Fragment<'uri>: TryFrom<TFragment, Error = TFragmentError>,
        FragmentError: From<TFragmentError>,
    {
        self.fragment = match fragment {
            Some(fragment) => Some(Fragment::try_from(fragment).map_err(|error| error)?),
            None => None,
        };
        Ok(self)
    }

    /// Sets the path part of the URI reference.
    ///
    /// If the given path is not a valid path (i.e. the conversion fails), an error is returned.
    ///
    /// It is required to specify a path. Not doing so will result in an error during the
    /// [`URIReferenceBuilder::build`] function.
    ///
    /// # Examples
    ///
    /// ```
    /// use uriparse::URIReferenceBuilder;
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .try_path("/my/path")
    ///     .unwrap();
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "/my/path");
    /// ```
    pub fn try_path<TPath, TPathError>(&mut self, path: TPath) -> Result<&mut Self, PathError>
    where
        Path<'uri>: TryFrom<TPath, Error = TPathError>,
        PathError: From<TPathError>,
    {
        self.path = Some(Path::try_from(path).map_err(|error| error)?);
        Ok(self)
    }

    /// Sets the query part of the URI reference.
    ///
    /// If the given query is not a valid query (i.e. the conversion fails), an error is returned.
    ///
    /// It is optional to specify a query.
    ///
    /// # Examples
    ///
    /// ```
    /// use uriparse::URIReferenceBuilder;
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .try_path("/my/path")
    ///     .unwrap()
    ///     .try_query(Some("query"))
    ///     .unwrap();
    /// let reference = builder.build().unwrap();
    /// assert_eq!(reference.to_string(), "/my/path?query");
    /// ```
    pub fn try_query<TQuery, TQueryError>(
        &mut self,
        query: Option<TQuery>,
    ) -> Result<&mut Self, QueryError>
    where
        Query<'uri>: TryFrom<TQuery, Error = TQueryError>,
        QueryError: From<TQueryError>,
    {
        self.query = match query {
            Some(query) => Some(Query::try_from(query).map_err(|error| error)?),
            None => None,
        };
        Ok(self)
    }

    /// Sets the scheme part of the URI reference.
    ///
    /// If the given scheme is not a valid scheme (i.e. the conversion fails), an error is returned.
    ///
    /// It is optional to specify a scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use uriparse::URIReferenceBuilder;
    ///
    /// let mut builder = URIReferenceBuilder::new();
    /// builder
    ///     .try_scheme(Some("urn"))
    ///     .unwrap()
    ///     .try_path("path")
    ///     .unwrap();
    /// let uri = builder.build().unwrap();
    /// assert_eq!(uri.to_string(), "urn:path");
    /// ```
    pub fn try_scheme<TScheme, TSchemeError>(
        &mut self,
        scheme: Option<TScheme>,
    ) -> Result<&mut Self, SchemeError>
    where
        Scheme<'uri>: TryFrom<TScheme, Error = TSchemeError>,
        SchemeError: From<TSchemeError>,
    {
        self.scheme = match scheme {
            Some(scheme) => Some(Scheme::try_from(scheme).map_err(|error| error)?),
            None => None,
        };
        Ok(self)
    }

    /// Consumes the builder and sets the authority part of the URI reference.
    ///
    /// It is optional to specify an authority.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Path, URIReferenceBuilder};
    ///
    /// let reference = URIReferenceBuilder::new()
    ///     .with_authority(Some(Authority::try_from("example.com").unwrap()))
    ///     .with_path(Path::try_from("/").unwrap())
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(reference.to_string(), "//example.com/")
    /// ```
    pub fn with_authority(mut self, authority: Option<Authority<'uri>>) -> Self {
        self.authority(authority);
        self
    }

    /// Consumes the builder and sets the fragment part of the URI reference.
    ///
    /// It is optional to specify a fragment.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Fragment, Path, URIReferenceBuilder};
    ///
    /// let reference = URIReferenceBuilder::new()
    ///     .with_path(Path::try_from("/").unwrap())
    ///     .with_fragment(Some(Fragment::try_from("fragment").unwrap()))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(reference.to_string(), "/#fragment")
    /// ```
    pub fn with_fragment(mut self, fragment: Option<Fragment<'uri>>) -> Self {
        self.fragment(fragment);
        self
    }

    /// Consumes the builder and sets the path part of the URI reference.
    ///
    /// It is required to specify a path. Not doing so will result in an error during the
    /// [`URIReferenceBuilder::build`] function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Path, URIReferenceBuilder};
    ///
    /// let reference = URIReferenceBuilder::new()
    ///     .with_path(Path::try_from("/").unwrap())
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(reference.to_string(), "/")
    /// ```
    pub fn with_path(mut self, path: Path<'uri>) -> Self {
        self.path(path);
        self
    }

    /// Consumes the builder and sets the query part of the URI reference.
    ///
    /// It is optional to specify a query.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Path, Query, URIReferenceBuilder};
    ///
    /// let reference = URIReferenceBuilder::new()
    ///     .with_path(Path::try_from("/").unwrap())
    ///     .with_query(Some(Query::try_from("query").unwrap()))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(reference.to_string(), "/?query")
    /// ```
    pub fn with_query(mut self, query: Option<Query<'uri>>) -> Self {
        self.query(query);
        self
    }

    /// Consumes the builder and sets the scheme part of the URI reference.
    ///
    /// It is optional to specify a scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Path, Scheme, URIReferenceBuilder};
    ///
    /// let reference = URIReferenceBuilder::new()
    ///     .with_scheme(Some(Scheme::HTTP))
    ///     .with_authority(Some(Authority::try_from("example.com").unwrap()))
    ///     .with_path(Path::try_from("/").unwrap())
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(reference.to_string(), "http://example.com/")
    /// ```
    pub fn with_scheme(mut self, scheme: Option<Scheme<'uri>>) -> Self {
        self.scheme(scheme);
        self
    }
}

/// An error representing an invalid URI reference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum URIReferenceError {
    /// Represents the case when there is no authority, but the first path segment starts with
    /// `"//"`. This is not allowed because it would be interpreted as an authority component.
    ///
    /// This can only occur when using creation functions that act on individual parts (e.g.
    /// [`URIReference::from_parts`]).
    AbsolutePathStartsWithTwoSlashes,

    /// The authority component of the relative reference was invalid.
    Authority(AuthorityError),

    /// The fragment component of the relative reference was invalid.
    Fragment(FragmentError),

    /// This error occurs when you do not specify a path component on the builder.
    ///
    /// This can only occur when using [`URIReferenceBuilder`].
    MissingPath,

    /// The path component of the relative reference was invalid.
    Path(PathError),

    /// The query component of the relative reference was invalid.
    Query(QueryError),

    /// The scheme component of the relative reference was invalid.
    Scheme(SchemeError),

    /// Represents the case when there is no authority, but the first path segment starts with
    /// `"//"`. This is not allowed because it would be interpreted as an authority component.
    ///
    /// This can only occur when using creation functions that act on individual parts (e.g.
    /// [`URIReference::from_parts`]).
    SchemelessPathStartsWithColonSegment,
}

impl Display for URIReferenceError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::URIReferenceError::*;

        match self {
            AbsolutePathStartsWithTwoSlashes => write!(
                formatter,
                "absolute path URI reference starts with two slashes"
            ),
            Authority(error) => error.fmt(formatter),
            Fragment(error) => error.fmt(formatter),
            Path(error) => error.fmt(formatter),
            Query(error) => error.fmt(formatter),
            Scheme(error) => error.fmt(formatter),
            MissingPath => write!(formatter, "URI reference missing path"),
            SchemelessPathStartsWithColonSegment => write!(
                formatter,
                "schemeless path URI reference starts with colon segment"
            ),
        }
    }
}

impl Error for URIReferenceError {}

impl From<Infallible> for URIReferenceError {
    fn from(_: Infallible) -> Self {
        URIReferenceError::AbsolutePathStartsWithTwoSlashes
    }
}

impl From<AuthorityError> for URIReferenceError {
    fn from(value: AuthorityError) -> Self {
        URIReferenceError::Authority(value)
    }
}

impl From<FragmentError> for URIReferenceError {
    fn from(value: FragmentError) -> Self {
        URIReferenceError::Fragment(value)
    }
}

impl From<PathError> for URIReferenceError {
    fn from(value: PathError) -> Self {
        URIReferenceError::Path(value)
    }
}

impl From<QueryError> for URIReferenceError {
    fn from(value: QueryError) -> Self {
        URIReferenceError::Query(value)
    }
}

impl From<SchemeError> for URIReferenceError {
    fn from(value: SchemeError) -> Self {
        URIReferenceError::Scheme(value)
    }
}

fn validate_absolute_path(
    authority: Option<&Authority>,
    path: &Path,
) -> Result<(), URIReferenceError> {
    if authority.is_some()
        || path.is_relative()
        || path.segments().len() == 1
        || !path.segments().first().unwrap().is_empty()
    {
        Ok(())
    } else {
        Err(URIReferenceError::AbsolutePathStartsWithTwoSlashes)
    }
}

fn validate_schemeless_path(
    scheme: Option<&Scheme>,
    authority: Option<&Authority>,
    path: &Path,
) -> Result<(), URIReferenceError> {
    if scheme.is_some()
        || authority.is_some()
        || !path
            .segments()
            .first()
            .unwrap()
            .bytes()
            .any(|byte| byte == b':')
    {
        Ok(())
    } else {
        Err(URIReferenceError::SchemelessPathStartsWithColonSegment)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_uri_reference() {
        let actual = URIReference::try_from("http://example.com").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            Some("example.com"),
            "/",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("http://example.com/").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            Some("example.com"),
            "/",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("http://example.com").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            Some("example.com"),
            "",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("http://example.com/").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            Some("example.com"),
            "",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("http:").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            None::<Authority>,
            "",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("http:/").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            None::<Authority>,
            "/",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("http:/path").unwrap();
        let expected = URIReference::from_parts(
            Some(Scheme::HTTP),
            None::<Authority>,
            "/path",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("//example.com/").unwrap();
        let expected = URIReference::from_parts(
            None::<Scheme>,
            Some("example.com"),
            "/",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("").unwrap();
        let expected = URIReference::from_parts(
            None::<Scheme>,
            None::<Authority>,
            "",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("*").unwrap();
        let expected = URIReference::from_parts(
            None::<Scheme>,
            None::<Authority>,
            "*",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("/").unwrap();
        let expected = URIReference::from_parts(
            None::<Scheme>,
            None::<Authority>,
            "/",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);

        let actual = URIReference::try_from("test/path").unwrap();
        let expected = URIReference::from_parts(
            None::<Scheme>,
            None::<Authority>,
            "test/path",
            None::<Query>,
            None::<Fragment>,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_uri_reference_error() {
        assert_eq!(
            URIReference::try_from("://www.example.com/"),
            Err(URIReferenceError::SchemelessPathStartsWithColonSegment)
        );
    }
}
