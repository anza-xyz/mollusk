//! Authority Component
//!
//! See [[RFC3986, Section 3.2](https://tools.ietf.org/html/rfc3986#section-3.2)].
//!
//! # Examples
//!
//! ```
//! use std::convert::TryFrom;
//!
//! use uriparse::Authority;
//!
//! let authority = Authority::try_from("example.com").unwrap();
//! let host = authority.into_parts().2;
//! let authority =
//!     Authority::from_parts(Some("username"), Some("password"), host, Some(80)).unwrap();
//! assert_eq!(authority.to_string(), "username:password@example.com:80");
//! ```
//!
//! # Equality
//!
//! While many components in this library support string comparison, [`Authority`] does not. This
//! comes down to it just being too expensive to do a proper host comparison. To do so would require
//! conversion to [`IpAddr`], which in the case of [`Ipv6Addr`] can be expensive.

use std::borrow::Cow;
use std::convert::{Infallible, TryFrom};
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ops::Deref;
use std::str;

use crate::utility::{
    get_percent_encoded_value, normalize_string, percent_encoded_equality, percent_encoded_hash,
    UNRESERVED_CHAR_MAP,
};

/// A map of byte characters that determines if a character is a valid IPv4 or registered name
/// character.
#[rustfmt::skip]
const IPV4_AND_REGISTERED_NAME_CHAR_MAP: [u8; 256] = [
 // 0     1     2     3     4     5     6     7     8     9     A     B     C     D     E     F
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 0
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 1
    0, b'!',    0,    0, b'$', b'%', b'&',b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.',    0, // 2
 b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',    0, b';',    0, b'=',    0,    0, // 3
    0, b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', // 4
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

/// A map of byte characters that determines if a character is a valid future IP literal character.
#[rustfmt::skip]
const IPV_FUTURE_CHAR_MAP: [u8; 256] = [
 // 0     1     2     3     4     5     6     7     8     9     A     B     C     D     E     F
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 0
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 1
    0, b'!',    0,    0, b'$',    0, b'&',b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.',    0, // 2
 b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b':', b';',    0, b'=',    0,    0, // 3
    0, b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', // 4
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

/// A map of byte characters that determines if a character is a valid user information character.
#[rustfmt::skip]
const USER_INFO_CHAR_MAP: [u8; 256] = [
 // 0     1     2     3     4     5     6     7     8     9     A     B     C     D     E     F
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 0
    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0, // 1
    0, b'!',    0,    0, b'$', b'%', b'&',b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.',    0, // 2
 b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b':', b';',    0, b'=',    0,    0, // 3
    0, b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', // 4
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

/// The authority component as defined in
/// [[RFC3986, Section 3.2](https://tools.ietf.org/html/rfc3986#section-3.2)].
///
/// Any conversions to a string will **not** hide the password component of the authority. Be
/// careful if you decide to perform logging.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Authority<'authority> {
    /// The host component of the authority as defined in
    /// [[RFC3986, Section 3.2.2](https://tools.ietf.org/html/rfc3986#section-3.2.2)].
    host: Host<'authority>,

    /// The password component of the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    password: Option<Password<'authority>>,

    /// The port component of the authority as defined in
    /// [[RFC3986, Section 3.2.3](https://tools.ietf.org/html/rfc3986#section-3.2.3)].
    port: Option<u16>,

    /// The username component of the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    username: Option<Username<'authority>>,
}

impl<'authority> Authority<'authority> {
    pub fn as_borrowed(&self) -> Authority {
        let host = match &self.host {
            Host::RegisteredName(name) => Host::RegisteredName(name.as_borrowed()),
            Host::IPv4Address(ipv4) => Host::IPv4Address(*ipv4),
            Host::IPv6Address(ipv6) => Host::IPv6Address(*ipv6),
        };
        let password = self.password.as_ref().map(Password::as_borrowed);
        let username = self.username.as_ref().map(Username::as_borrowed);

        Authority {
            host,
            password,
            port: self.port,
            username,
        }
    }

    /// Constructs a new [`Authority`] from the individual parts: username, password, host, and
    /// port.
    ///
    /// The lifetime used by the resulting value will be the lifetime of the part that is most
    /// restricted in scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::from_parts(
    ///     Some("username"),
    ///     Some("password"),
    ///     "example.com",
    ///     Some(80)
    /// ).unwrap();
    /// assert_eq!(authority.to_string(), "username:password@example.com:80");
    /// ```
    pub fn from_parts<
        'new_authority,
        TUsername,
        TPassword,
        THost,
        TUsernameError,
        TPasswordError,
        THostError,
    >(
        username: Option<TUsername>,
        password: Option<TPassword>,
        host: THost,
        port: Option<u16>,
    ) -> Result<Authority<'new_authority>, AuthorityError>
    where
        Username<'new_authority>: TryFrom<TUsername, Error = TUsernameError>,
        Password<'new_authority>: TryFrom<TPassword, Error = TPasswordError>,
        Host<'new_authority>: TryFrom<THost, Error = THostError>,
        AuthorityError: From<TUsernameError> + From<TPasswordError> + From<THostError>,
    {
        let username = match username {
            Some(username) => Some(Username::try_from(username)?),
            None => None,
        };

        let password = match password {
            Some(password) => Some(Password::try_from(password)?),
            None => None,
        };

        let host = Host::try_from(host)?;

        Ok(Authority {
            host,
            password,
            port,
            username,
        })
    }

    /// Returns whether there is a password in the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    ///
    /// There will only be a password if the URI has a user information component *and* the
    /// component contains the `':'` delimiter.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username:password@example.com").unwrap();
    /// assert!(authority.has_password());
    /// ```
    pub fn has_password(&self) -> bool {
        self.password.is_some()
    }

    /// Returns whether there is a password in the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("example.com:8080").unwrap();
    /// assert!(authority.has_port());
    /// ```
    pub fn has_port(&self) -> bool {
        self.port.is_some()
    }

    /// Returns whether there is a username in the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    ///
    /// There will *always* be a username as long as there is a `'@'` delimiter present in the
    /// authority.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username@example.com").unwrap();
    /// assert!(authority.has_username());
    /// ```
    pub fn has_username(&self) -> bool {
        self.username.is_some()
    }

    /// The host component of the authority as defined in
    /// [[RFC3986, Section 3.2.2](https://tools.ietf.org/html/rfc3986#section-3.2.2)].
    ///
    /// An authority component always has a host, though it may be an empty registered name.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username:password@example.com").unwrap();
    /// assert_eq!(authority.host().to_string().as_str(), "example.com");
    /// ```
    pub fn host(&self) -> &Host<'authority> {
        &self.host
    }

    /// Converts the [`Authority`] into an owned copy.
    ///
    /// If you construct the authority from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the authority will just copy the eferences, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Authority<'static> {
        let password = self.password.map(Password::into_owned);
        let username = self.username.map(Username::into_owned);
        let host = match self.host {
            Host::RegisteredName(name) => Host::RegisteredName(name.into_owned()),
            Host::IPv4Address(ipv4) => Host::IPv4Address(ipv4),
            Host::IPv6Address(ipv6) => Host::IPv6Address(ipv6),
        };

        Authority {
            host,
            port: self.port,
            password,
            username,
        }
    }

    /// Consumes the [`Authority`] and returns its parts: username, password, host, and port.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username:password@example.com:80").unwrap();
    /// let (username, password, host, port) = authority.into_parts();
    ///
    /// assert_eq!(username.unwrap(), "username");
    /// assert_eq!(password.unwrap(), "password");
    /// assert_eq!(host.to_string(), "example.com");
    /// assert_eq!(port.unwrap(), 80);
    /// ```
    pub fn into_parts(
        self,
    ) -> (
        Option<Username<'authority>>,
        Option<Password<'authority>>,
        Host<'authority>,
        Option<u16>,
    ) {
        (self.username, self.password, self.host, self.port)
    }

    /// Returns whether the authority is normalized.
    ///
    /// A normalized authority will have all of its sub-components normalized.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username:password@example.com").unwrap();
    /// assert!(authority.is_normalized());
    ///
    /// let mut authority = Authority::try_from("username:p%61ssword@EXAMPLE.COM").unwrap();
    /// assert!(!authority.is_normalized());
    /// authority.normalize();
    /// assert!(authority.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        if let Some(username) = self.username.as_ref() {
            if !username.is_normalized() {
                return false;
            }
        }

        if let Some(password) = self.password.as_ref() {
            if !password.is_normalized() {
                return false;
            }
        }

        self.host.is_normalized()
    }

    /// Maps the host using the given map function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Host};
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.map_host(|_| Host::try_from("127.0.0.1").unwrap());
    /// assert_eq!(authority.to_string(), "127.0.0.1");
    /// ```
    pub fn map_host<TMapper>(&mut self, mapper: TMapper) -> &Host<'authority>
    where
        TMapper: FnOnce(Host<'authority>) -> Host<'authority>,
    {
        let temp_host = Host::RegisteredName(RegisteredName {
            normalized: true,
            registered_name: Cow::from(""),
        });
        let host = mapper(mem::replace(&mut self.host, temp_host));
        self.set_host(host)
            .expect("mapped host resulted in invalid state")
    }

    /// Maps the password using the given map function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Password};
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.map_password(|_| Some(Password::try_from("password").unwrap()));
    /// assert_eq!(authority.to_string(), ":password@example.com");
    /// ```
    pub fn map_password<TMapper>(&mut self, mapper: TMapper) -> Option<&Password<'authority>>
    where
        TMapper: FnOnce(Option<Password<'authority>>) -> Option<Password<'authority>>,
    {
        let password = mapper(self.password.take());
        self.set_password(password)
            .expect("mapped password resulted in invalid state")
    }

    /// Maps the port using the given map function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.map_port(|_| Some(8080));
    /// assert_eq!(authority.to_string(), "example.com:8080");
    /// ```
    pub fn map_port<TMapper>(&mut self, mapper: TMapper) -> Option<u16>
    where
        TMapper: FnOnce(Option<u16>) -> Option<u16>,
    {
        let port = mapper(self.port);
        self.set_port(port)
    }

    /// Maps the username using the given map function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Username};
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.map_username(|_| Some(Username::try_from("username").unwrap()));
    /// assert_eq!(authority.to_string(), "username@example.com");
    /// ```
    pub fn map_username<TMapper>(&mut self, mapper: TMapper) -> Option<&Username<'authority>>
    where
        TMapper: FnOnce(Option<Username<'authority>>) -> Option<Username<'authority>>,
    {
        let username = mapper(self.username.take());
        self.set_username(username)
            .expect("mapped username resulted in invalid state")
    }

    /// Normalizes the authority.
    ///
    /// A normalized authority will have all of its sub-components normalized.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let mut authority = Authority::try_from("username:password@example.com").unwrap();
    /// authority.normalize();
    /// assert_eq!(authority.to_string(), "username:password@example.com");
    ///
    /// let mut authority = Authority::try_from("username:p%61ssword@EXAMPLE.COM").unwrap();
    /// assert_eq!(authority.to_string(), "username:p%61ssword@EXAMPLE.COM");
    /// authority.normalize();
    /// assert_eq!(authority.to_string(), "username:password@example.com");
    /// ```
    pub fn normalize(&mut self) {
        if let Some(username) = self.username.as_mut() {
            username.normalize();
        }

        if let Some(password) = self.password.as_mut() {
            password.normalize();
        }

        self.host.normalize();
    }

    /// The password component of the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    ///
    /// The password will be `None` if the user information component of the authority did not
    /// contain a `':'`. Otherwise, it will be whatever is after the `':'` until the `'@'`
    /// character. It may be empty as well.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username:password@example.com").unwrap();
    /// assert_eq!(authority.password().unwrap(), "password");
    /// ```
    pub fn password(&self) -> Option<&Password<'authority>> {
        self.password.as_ref()
    }

    /// The port component of the authority as defined in
    /// [[RFC3986, Section 3.2.3]](https://tools.ietf.org/html/rfc3986#section-3.2.3).
    ///
    /// The port will be `None` if a port was not specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("example.com:80").unwrap();
    /// assert_eq!(authority.port().unwrap(), 80);
    /// ```
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Sets the host of the authority.
    ///
    /// An error will be returned if the conversion to a [`Host`] fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use std::net::Ipv6Addr;
    ///
    /// use uriparse::{Authority, Host};
    ///
    /// let mut authority = Authority::try_from("example.com:8080").unwrap();
    /// authority.set_host("127.0.0.1");
    /// assert_eq!(authority.to_string(), "127.0.0.1:8080");
    /// authority.set_host(Host::IPv6Address("::1".parse().unwrap()));
    /// assert_eq!(authority.to_string(), "[::1]:8080");
    /// ```
    pub fn set_host<THost, THostError>(
        &mut self,
        host: THost,
    ) -> Result<&Host<'authority>, AuthorityError>
    where
        Host<'authority>: TryFrom<THost, Error = THostError>,
        AuthorityError: From<THostError>,
    {
        self.host = Host::try_from(host)?;
        Ok(self.host())
    }

    /// Sets the password of the authority.
    ///
    /// An error will be returned if the conversion to a [`Password`] fails.
    ///
    /// If the given password is not `None`, then the username will be set to `""` if it is
    /// currently not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.set_password(Some("secret"));
    /// assert_eq!(authority.to_string(), ":secret@example.com");
    /// ```
    pub fn set_password<TPassword, TPasswordError>(
        &mut self,
        password: Option<TPassword>,
    ) -> Result<Option<&Password<'authority>>, AuthorityError>
    where
        Password<'authority>: TryFrom<TPassword, Error = TPasswordError>,
        AuthorityError: From<TPasswordError>,
    {
        self.password = match password {
            Some(password) => {
                let password = Password::try_from(password)?;

                if self.username.is_none() {
                    self.username = Some(Username {
                        normalized: true,
                        username: Cow::from(""),
                    });
                }

                Some(password)
            }
            None => None,
        };
        Ok(self.password())
    }

    /// Sets the port of the authority.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.set_port(Some(8080));
    /// assert_eq!(authority.to_string(), "example.com:8080");
    /// ```
    pub fn set_port(&mut self, port: Option<u16>) -> Option<u16> {
        self.port = port;
        self.port
    }

    /// Sets the username of the authority.
    ///
    /// An error will be returned if the conversion to a [`Username`] fails.
    ///
    /// If the given username is `None`, this will also remove any set password.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::{Authority, Username};
    ///
    /// let mut authority = Authority::try_from("example.com").unwrap();
    /// authority.set_username(Some("myname"));
    /// assert_eq!(authority.to_string(), "myname@example.com");
    ///
    /// let mut authority = Authority::try_from("user:pass@example.com").unwrap();
    /// authority.set_username(None::<Username>);
    /// assert_eq!(authority.to_string(), "example.com");
    /// ```
    pub fn set_username<TUsername, TUsernameError>(
        &mut self,
        username: Option<TUsername>,
    ) -> Result<Option<&Username<'authority>>, AuthorityError>
    where
        Username<'authority>: TryFrom<TUsername, Error = TUsernameError>,
        AuthorityError: From<TUsernameError>,
    {
        self.username = match username {
            Some(username) => Some(Username::try_from(username)?),
            None => {
                self.password = None;
                None
            }
        };
        Ok(self.username())
    }

    /// The username component of the authority as defined in
    /// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
    ///
    /// The username will be `None` if the user information component of the authority did not
    /// contain a `':'`. Otherwise, it will be whatever is after the `':'` until the `'@'`
    /// character. It may be empty as well.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Authority;
    ///
    /// let authority = Authority::try_from("username:password@example.com").unwrap();
    /// assert_eq!(authority.password().unwrap(), "password");
    /// ```
    pub fn username(&self) -> Option<&Username<'authority>> {
        self.username.as_ref()
    }
}

impl Display for Authority<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if let Some(ref username) = self.username {
            username.fmt(formatter)?;

            if let Some(ref password) = self.password {
                formatter.write_char(':')?;
                password.fmt(formatter)?;
            }

            formatter.write_char('@')?;
        }

        self.host.fmt(formatter)?;

        if let Some(port) = self.port {
            formatter.write_char(':')?;
            port.fmt(formatter)?;
        }

        Ok(())
    }
}

impl<'authority> From<Authority<'authority>> for String {
    fn from(value: Authority<'authority>) -> String {
        value.to_string()
    }
}

impl<'authority> TryFrom<&'authority [u8]> for Authority<'authority> {
    type Error = AuthorityError;

    fn try_from(value: &'authority [u8]) -> Result<Self, Self::Error> {
        let (authority, rest) = parse_authority(value)?;

        if rest.is_empty() {
            Ok(authority)
        } else if authority.has_port() {
            Err(AuthorityError::Port(PortError::InvalidCharacter))
        } else if authority.host().is_ipv6_address() {
            Err(AuthorityError::Host(HostError::InvalidIPv6Character))
        } else {
            Err(AuthorityError::Host(
                HostError::InvalidIPv4OrRegisteredNameCharacter,
            ))
        }
    }
}

impl<'authority> TryFrom<&'authority str> for Authority<'authority> {
    type Error = AuthorityError;

    fn try_from(value: &'authority str) -> Result<Self, Self::Error> {
        Authority::try_from(value.as_bytes())
    }
}

/// The host component of the authority as defined in
/// [[RFC3986, Section 3.2.2](https://tools.ietf.org/html/rfc3986#section-3.2.2)].
///
/// The RFC mentions support for future IP address literals. Of course, as of this moment there
/// exist none, so hosts of the form `"[v*...]"` where `'*'` is a hexadecimal digit and `'...'` is
/// the actual IP literal are not considered valid.
///
/// Also, the host is case-insensitive meaning that `"example.com"` and `"ExAmPlE.CoM"` refer to the
/// same host. Furthermore, percent-encoding plays no role in equality checking for characters in
/// the unreserved character set meaning that `"example.com"` and `"ex%61mple.com"` are identical.
/// Both of these attributes are reflected in the equality and hash functions.
///
/// However, be aware that just because percent-encoding plays no role in equality checking does not
/// mean that the host is normalized. If the host needs to be normalized, use the
/// [`Host::normalize`] function.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Host<'host> {
    /// An IPv4 address. Based on the `std`'s implementation, leading zeros for octets are allowed
    /// for up to three digits. So for example, `"000.000.000.000"` is still considered a valid IPv4
    /// address, but `"000.000.000.0000"` is not. Thus, it would be considered a registered name.
    IPv4Address(Ipv4Addr),

    /// An IPv6 address. This will always be encased in brackets (`'['` and `']'`).
    IPv6Address(Ipv6Addr),

    /// Any other host that does not follow the syntax of an IP address. This includes even hosts of
    /// the form `"999.999.999.999"`. One might expect this to produce an invalid IPv4 error, but
    /// the RFC states that it is a "first-match-wins" algorithm, and that host does not match the
    /// IPv4 literal syntax.
    ///
    /// This may be changed in the future, since arguments can be made from either side.
    RegisteredName(RegisteredName<'host>),
}

impl Host<'_> {
    /// Returns a new host which is identical but has a lifetime tied to this host.
    pub fn as_borrowed(&self) -> Host {
        use self::Host::*;

        match self {
            IPv4Address(ipv4) => IPv4Address(*ipv4),
            IPv6Address(ipv6) => IPv6Address(*ipv6),
            RegisteredName(name) => RegisteredName(name.as_borrowed()),
        }
    }

    /// Converts the [`Host`] into an owned copy.
    ///
    /// If you construct the host from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the host will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Host<'static> {
        use self::Host::*;

        match self {
            IPv4Address(ipv4) => IPv4Address(ipv4),
            IPv6Address(ipv6) => IPv6Address(ipv6),
            RegisteredName(name) => RegisteredName(name.into_owned()),
        }
    }

    /// Returns whether the host is an IPv4 address.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Host;
    ///
    /// let host = Host::try_from("192.168.1.1").unwrap();
    /// assert!(host.is_ipv4_address());
    /// ```
    pub fn is_ipv4_address(&self) -> bool {
        match self {
            Host::IPv4Address(_) => true,
            _ => false,
        }
    }

    /// Returns whether the host is an IPv6 address.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Host;
    ///
    /// let host = Host::try_from("[::1]").unwrap();
    /// assert!(host.is_ipv6_address());
    /// ```
    pub fn is_ipv6_address(&self) -> bool {
        match self {
            Host::IPv6Address(_) => true,
            _ => false,
        }
    }

    /// Returns whether the host is normalized.
    ///
    /// IPv4 and IPv6 hosts will always be normalized. Registered names are considered normalized
    /// if all characters are lowercase, no bytes that are in the unreserved character set are
    /// percent-encoded, and all alphabetical characters in percent-encodings are uppercase.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Host;
    ///
    /// let host = Host::try_from("192.168.1.1").unwrap();
    /// assert!(host.is_normalized());
    ///
    /// let mut host = Host::try_from("EXAMPLE.COM").unwrap();
    /// assert!(!host.is_normalized());
    /// host.normalize();
    /// assert!(host.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        match self {
            Host::RegisteredName(name) => name.is_normalized(),
            _ => true,
        }
    }

    /// Returns whether the host is a registered name.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Host;
    ///
    /// let host = Host::try_from("example.com").unwrap();
    /// assert!(host.is_registered_name());
    /// ```
    pub fn is_registered_name(&self) -> bool {
        match self {
            Host::RegisteredName(_) => true,
            _ => false,
        }
    }

    /// Normalizes the host such that all characters are lowercase, no bytes that are in the
    /// unreserved character set are percent-encoded, and all alphabetical characters in
    /// percent-encodings are uppercase.
    ///
    /// If the host is already normalized, the function will return immediately. Otherwise, if
    /// the host is not owned, this function will perform an allocation to clone it. The
    /// normalization itself though, is done in-place with no extra memory allocations required.
    ///
    /// IPv4 and IPv6 hosts are always considered normalized.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Host;
    ///
    /// let mut host = Host::try_from("192.168.1.1").unwrap();
    /// host.normalize();
    /// assert_eq!(host.to_string(), "192.168.1.1");
    ///
    /// let mut host = Host::try_from("%ff%41").unwrap();
    /// assert_eq!(host.to_string(), "%ff%41");
    /// host.normalize();
    /// assert_eq!(host.to_string(), "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if let Host::RegisteredName(name) = self {
            name.normalize()
        }
    }
}

impl Display for Host<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::Host::*;

        match self {
            IPv4Address(address) => address.fmt(formatter),
            IPv6Address(address) => {
                formatter.write_char('[')?;
                address.fmt(formatter)?;
                formatter.write_char(']')
            }
            RegisteredName(name) => formatter.write_str(name.as_str()),
        }
    }
}

impl<'host> From<Host<'host>> for String {
    fn from(value: Host<'host>) -> String {
        value.to_string()
    }
}

impl From<IpAddr> for Host<'static> {
    fn from(value: IpAddr) -> Self {
        match value {
            IpAddr::V4(address) => Host::IPv4Address(address),
            IpAddr::V6(address) => Host::IPv6Address(address),
        }
    }
}

impl From<Ipv4Addr> for Host<'static> {
    fn from(value: Ipv4Addr) -> Self {
        Host::IPv4Address(value)
    }
}

impl From<Ipv6Addr> for Host<'static> {
    fn from(value: Ipv6Addr) -> Self {
        Host::IPv6Address(value)
    }
}

impl<'host> TryFrom<&'host [u8]> for Host<'host> {
    type Error = HostError;

    fn try_from(value: &'host [u8]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            let registered_name = RegisteredName {
                normalized: true,
                registered_name: Cow::from(""),
            };
            return Ok(Host::RegisteredName(registered_name));
        }

        match (value.get(0), value.get(value.len() - 1)) {
            (Some(b'['), Some(b']')) => {
                match value.get(1..3) {
                    Some(&[prefix, version])
                        if prefix.to_ascii_lowercase() == b'v' && version.is_ascii_hexdigit() =>
                    {
                        // IPvFuture

                        let ipvfuture = &value[3..value.len() - 1];

                        if check_ipvfuture(ipvfuture) {
                            return Err(HostError::AddressMechanismNotSupported);
                        } else {
                            return Err(HostError::InvalidIPvFutureCharacter);
                        }
                    }
                    _ => (),
                }

                // IPv6

                let ipv6 = &value[1..value.len() - 1];

                if !check_ipv6(ipv6) {
                    return Err(HostError::InvalidIPv6Character);
                }

                // Unsafe: The function above [`check_ipv6`] ensures this is valid ASCII-US.
                let ipv6: Ipv6Addr = unsafe { str::from_utf8_unchecked(ipv6) }
                    .parse()
                    .map_err(|_| HostError::InvalidIPv6Format)?;
                Ok(Host::IPv6Address(ipv6))
            }
            _ => {
                let (valid, normalized) = check_ipv4_or_registered_name(value);

                if valid {
                    // Unsafe: The function above [`check_ipv4_or_registered_name`] ensures
                    // this is valid ASCII-US.
                    let value_string = unsafe { str::from_utf8_unchecked(value) };

                    match value_string.parse() {
                        Ok(ipv4) => Ok(Host::IPv4Address(ipv4)),
                        Err(_) => Ok(Host::RegisteredName(RegisteredName {
                            normalized,
                            registered_name: Cow::from(value_string),
                        })),
                    }
                } else {
                    Err(HostError::InvalidIPv4OrRegisteredNameCharacter)
                }
            }
        }
    }
}

impl<'host> TryFrom<&'host str> for Host<'host> {
    type Error = HostError;

    fn try_from(value: &'host str) -> Result<Self, Self::Error> {
        Host::try_from(value.as_bytes())
    }
}

/// The password component of the authority as defined in
/// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
///
/// Even though this library supports parsing the password from the user information, it should be
/// noted that the format "username:password" is deprecated. Also, be careful logging this!
///
/// The password is case-sensitive. Furthermore, percent-encoding plays no role in equality checking
/// for characters in the unreserved character set meaning that `"password"` and `"p%61ssword"` are
/// identical. Both of these attributes are reflected in the equality and hash functions.
///
/// Be aware that just because percent-encoding plays no role in equality checking does not
/// mean that the password is normalized. If the password needs to be normalized, use the
/// [`Password::normalize`] function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Password<'password> {
    /// Whether the password is normalized.
    normalized: bool,

    /// The internal password source that is either owned or borrowed.
    password: Cow<'password, str>,
}

impl Password<'_> {
    /// Returns a new password which is identical but has a lifetime tied to this password.
    pub fn as_borrowed(&self) -> Password {
        use self::Cow::*;

        let password = match &self.password {
            Borrowed(borrowed) => *borrowed,
            Owned(owned) => owned.as_str(),
        };

        Password {
            normalized: self.normalized,
            password: Cow::Borrowed(password),
        }
    }

    /// Returns a `str` representation of the password.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Password;
    ///
    /// let password = Password::try_from("password").unwrap();
    /// assert_eq!(password, "password");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.password
    }

    /// Converts the [`Password`] into an owned copy.
    ///
    /// If you construct the authority from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the password will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Password<'static> {
        Password {
            normalized: self.normalized,
            password: Cow::from(self.password.into_owned()),
        }
    }

    /// Returns whether the password is normalized.
    ///
    /// A normalized password will have no bytes that are in the unreserved character set
    /// percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Password;
    ///
    /// let password = Password::try_from("password").unwrap();
    /// assert!(password.is_normalized());
    ///
    /// let mut password = Password::try_from("%ff%ff").unwrap();
    /// assert!(!password.is_normalized());
    /// password.normalize();
    /// assert!(password.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    /// Normalizes the password such that it will have no bytes that are in the unreserved character
    /// set percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// If the password is already normalized, the function will return immediately. Otherwise, if
    /// the password is not owned, this function will perform an allocation to clone it. The
    /// normalization itself though, is done in-place with no extra memory allocations required.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Password;
    ///
    /// let mut password = Password::try_from("password").unwrap();
    /// password.normalize();
    /// assert_eq!(password, "password");
    ///
    /// let mut password = Password::try_from("%ff%41").unwrap();
    /// assert_eq!(password, "%ff%41");
    /// password.normalize();
    /// assert_eq!(password, "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if !self.normalized {
            // Unsafe: Passwords must be valid ASCII-US, so this is safe.
            unsafe { normalize_string(&mut self.password.to_mut(), true) };
            self.normalized = true;
        }
    }
}

impl AsRef<[u8]> for Password<'_> {
    fn as_ref(&self) -> &[u8] {
        self.password.as_bytes()
    }
}

impl AsRef<str> for Password<'_> {
    fn as_ref(&self) -> &str {
        &self.password
    }
}

impl Deref for Password<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.password
    }
}

impl Display for Password<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(&self.password)
    }
}

impl Eq for Password<'_> {}

impl<'password> From<Password<'password>> for String {
    fn from(value: Password<'password>) -> String {
        value.to_string()
    }
}

impl Hash for Password<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        percent_encoded_hash(self.password.as_bytes(), state, true);
    }
}

impl PartialEq for Password<'_> {
    fn eq(&self, other: &Password) -> bool {
        *self == *other.as_bytes()
    }
}

impl PartialEq<[u8]> for Password<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        percent_encoded_equality(self.password.as_bytes(), other, true)
    }
}

impl<'password> PartialEq<Password<'password>> for [u8] {
    fn eq(&self, other: &Password<'password>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for Password<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'password> PartialEq<Password<'password>> for &'a [u8] {
    fn eq(&self, other: &Password<'password>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for Password<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'password> PartialEq<Password<'password>> for str {
    fn eq(&self, other: &Password<'password>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Password<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'password> PartialEq<Password<'password>> for &'a str {
    fn eq(&self, other: &Password<'password>) -> bool {
        other == self.as_bytes()
    }
}

impl<'password> TryFrom<&'password [u8]> for Password<'password> {
    type Error = PasswordError;

    fn try_from(value: &'password [u8]) -> Result<Self, Self::Error> {
        let normalized = check_user_info(value, false)?;

        // Unsafe: The function above [`check_user_info`] ensures this is valid ASCII-US.
        Ok(Password {
            normalized,
            password: Cow::from(unsafe { str::from_utf8_unchecked(value) }),
        })
    }
}

impl<'password> TryFrom<&'password str> for Password<'password> {
    type Error = PasswordError;

    fn try_from(value: &'password str) -> Result<Self, Self::Error> {
        Password::try_from(value.as_bytes())
    }
}

/// A host that is a registered name (i.e. not an IP literal).
///
/// The registered name is case-insensitive meaning that `"example.com"` and `"ExAmPlE.CoM"` refer
/// to the same registered name. Furthermore, percent-encoding plays no role in equality checking
/// for characters in the unreserved character set meaning that `"example.com"` and
/// `"ex%61mple.com"` are identical. Both of these attributes are reflected in the equality and hash
/// functions.
///
/// However, be aware that just because percent-encoding plays no role in equality checking does not
/// mean that the host is normalized. If the registered name needs to be normalized, use the
/// [`RegisteredName::normalize`] function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RegisteredName<'name> {
    /// Whether the registered name is normalized.
    normalized: bool,

    /// The internal registered name source that is either owned or borrowed.
    registered_name: Cow<'name, str>,
}

impl RegisteredName<'_> {
    /// Returns a new registered name which is identical but has a lifetime tied to this registered
    /// name.
    pub fn as_borrowed(&self) -> RegisteredName {
        use self::Cow::*;

        let name = match &self.registered_name {
            Borrowed(borrowed) => *borrowed,
            Owned(owned) => owned.as_str(),
        };

        RegisteredName {
            normalized: self.normalized,
            registered_name: Cow::Borrowed(name),
        }
    }

    /// Returns a `str` representation of the registered name.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::RegisteredName;
    ///
    /// let name = RegisteredName::try_from("example.com").unwrap();
    /// assert_eq!(name.as_str(), "example.com");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.registered_name
    }

    /// Converts the [`RegisteredName`] into an owned copy.
    ///
    /// If you construct the registered name from a source with a non-static lifetime, you may run
    /// into lifetime problems due to the way the struct is designed. Calling this function will
    /// ensure that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the registered name will just copy the
    /// references, and thus the lifetime will remain the same.
    pub fn into_owned(self) -> RegisteredName<'static> {
        RegisteredName {
            normalized: self.normalized,
            registered_name: Cow::from(self.registered_name.into_owned()),
        }
    }

    /// Returns whether the registered name is normalized.
    ///
    /// Registered names are considered normalized if all characters are lowercase, no bytes that
    /// are in the unreserved character set are percent-encoded, and all alphabetical characters in
    /// percent-encodings are uppercase.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::RegisteredName;
    ///
    /// let name = RegisteredName::try_from("example.com").unwrap();
    /// assert!(name.is_normalized());
    ///
    /// let mut name = RegisteredName::try_from("EXAMPLE.COM").unwrap();
    /// assert!(!name.is_normalized());
    /// name.normalize();
    /// assert!(name.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    /// Normalizes the registered name such that all characters are lowercase, no bytes that are in
    /// the unreserved character set are percent-encoded, and all alphabetical characters in
    /// percent-encodings are uppercase.
    ///
    /// If the registered name is already normalized, the function will return immediately.
    /// Otherwise, if the registered name is not owned, this function will perform an allocation to
    /// clone it. The normalization itself though, is done in-place with no extra memory allocations
    /// required.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::RegisteredName;
    ///
    /// let mut name = RegisteredName::try_from("example.com").unwrap();
    /// name.normalize();
    /// assert_eq!(name.to_string(), "example.com");
    ///
    /// let mut name = RegisteredName::try_from("%ff%41").unwrap();
    /// assert_eq!(name.to_string(), "%ff%41");
    /// name.normalize();
    /// assert_eq!(name.to_string(), "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if !self.normalized {
            // Unsafe: Registered names must be valid ASCII-US, so this is safe.
            unsafe { normalize_string(&mut self.registered_name.to_mut(), false) };
            self.normalized = true;
        }
    }
}

impl AsRef<[u8]> for RegisteredName<'_> {
    fn as_ref(&self) -> &[u8] {
        self.registered_name.as_bytes()
    }
}

impl AsRef<str> for RegisteredName<'_> {
    fn as_ref(&self) -> &str {
        &self.registered_name
    }
}

impl Deref for RegisteredName<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.registered_name
    }
}

impl Display for RegisteredName<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(&self.registered_name)
    }
}

impl Eq for RegisteredName<'_> {}

impl<'name> From<RegisteredName<'name>> for String {
    fn from(value: RegisteredName<'name>) -> String {
        value.to_string()
    }
}

impl Hash for RegisteredName<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        percent_encoded_hash(self.registered_name.as_bytes(), state, false);
    }
}

impl PartialEq for RegisteredName<'_> {
    fn eq(&self, other: &RegisteredName) -> bool {
        *self == *other.as_bytes()
    }
}

impl PartialEq<[u8]> for RegisteredName<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        percent_encoded_equality(self.registered_name.as_bytes(), other, false)
    }
}

impl<'name> PartialEq<RegisteredName<'name>> for [u8] {
    fn eq(&self, other: &RegisteredName<'name>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for RegisteredName<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'name> PartialEq<RegisteredName<'name>> for &'a [u8] {
    fn eq(&self, other: &RegisteredName<'name>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for RegisteredName<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'name> PartialEq<RegisteredName<'name>> for str {
    fn eq(&self, other: &RegisteredName<'name>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for RegisteredName<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'name> PartialEq<RegisteredName<'name>> for &'a str {
    fn eq(&self, other: &RegisteredName<'name>) -> bool {
        other == self.as_bytes()
    }
}

impl<'name> TryFrom<&'name [u8]> for RegisteredName<'name> {
    type Error = RegisteredNameError;

    fn try_from(value: &'name [u8]) -> Result<Self, Self::Error> {
        match Host::try_from(value) {
            Ok(Host::RegisteredName(name)) => Ok(name),
            _ => Err(RegisteredNameError),
        }
    }
}

impl<'name> TryFrom<&'name str> for RegisteredName<'name> {
    type Error = RegisteredNameError;

    fn try_from(value: &'name str) -> Result<Self, Self::Error> {
        RegisteredName::try_from(value.as_bytes())
    }
}

/// The username component of the authority as defined in
/// [[RFC3986, Section 3.2.1](https://tools.ietf.org/html/rfc3986#section-3.2.1)].
///
/// The username is case-sensitive. Furthermore, percent-encoding plays no role in equality checking
/// for characters in the unreserved character set meaning that `"username"` and `"usern%61me"` are
/// identical. Both of these attributes are reflected in the equality and hash functions.
///
/// Be aware that just because percent-encoding plays no role in equality checking does not
/// mean that the username is normalized. If the username needs to be normalized, use the
/// [`Username::normalize`] function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Username<'username> {
    /// Whether the username is normalized.
    normalized: bool,

    /// The internal username source that is either owned or borrowed.
    username: Cow<'username, str>,
}

impl Username<'_> {
    /// Returns a new username which is identical but has a lifetime tied to this username.
    pub fn as_borrowed(&self) -> Username {
        use self::Cow::*;

        let username = match &self.username {
            Borrowed(borrowed) => *borrowed,
            Owned(owned) => owned.as_str(),
        };

        Username {
            normalized: self.normalized,
            username: Cow::Borrowed(username),
        }
    }

    /// Returns a `str` representation of the username.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Username;
    ///
    /// let username = Username::try_from("username").unwrap();
    /// assert_eq!(username.as_str(), "username");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.username
    }

    /// Converts the [`Username`] into an owned copy.
    ///
    /// If you construct the username from a source with a non-static lifetime, you may run into
    /// lifetime problems due to the way the struct is designed. Calling this function will ensure
    /// that the returned value has a static lifetime.
    ///
    /// This is different from just cloning. Cloning the username will just copy the references, and
    /// thus the lifetime will remain the same.
    pub fn into_owned(self) -> Username<'static> {
        Username {
            normalized: self.normalized,
            username: Cow::from(self.username.into_owned()),
        }
    }

    /// Returns whether the username is normalized.
    ///
    /// A normalized username will have no bytes that are in the unreserved character set
    /// percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// This function runs in constant-time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Username;
    ///
    /// let username = Username::try_from("username").unwrap();
    /// assert!(username.is_normalized());
    ///
    /// let mut username = Username::try_from("%ff%ff").unwrap();
    /// assert!(!username.is_normalized());
    /// username.normalize();
    /// assert!(username.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    /// Normalizes the username such that it will have no bytes that are in the unreserved character
    /// set percent-encoded and all alphabetical characters in percent-encodings will be uppercase.
    ///
    /// If the username is already normalized, the function will return immediately. Otherwise, if
    /// the username is not owned, this function will perform an allocation to clone it. The
    /// normalization itself though, is done in-place with no extra memory allocations required.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// use uriparse::Username;
    ///
    /// let mut username = Username::try_from("username").unwrap();
    /// username.normalize();
    /// assert_eq!(username, "username");
    ///
    /// let mut username = Username::try_from("%ff%41").unwrap();
    /// assert_eq!(username, "%ff%41");
    /// username.normalize();
    /// assert_eq!(username, "%FFA");
    /// ```
    pub fn normalize(&mut self) {
        if !self.normalized {
            // Unsafe: Usernames must be valid ASCII-US, so this is safe.
            unsafe { normalize_string(&mut self.username.to_mut(), true) };
            self.normalized = true;
        }
    }
}

impl AsRef<[u8]> for Username<'_> {
    fn as_ref(&self) -> &[u8] {
        self.username.as_bytes()
    }
}

impl AsRef<str> for Username<'_> {
    fn as_ref(&self) -> &str {
        &self.username
    }
}

impl Deref for Username<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.username
    }
}

impl Display for Username<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(&self.username)
    }
}

impl<'username> Eq for Username<'username> {}

impl<'username> From<Username<'username>> for String {
    fn from(value: Username<'username>) -> String {
        value.to_string()
    }
}

impl Hash for Username<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        percent_encoded_hash(self.username.as_bytes(), state, true);
    }
}

impl PartialEq for Username<'_> {
    fn eq(&self, other: &Username) -> bool {
        *self == *other.as_bytes()
    }
}

impl PartialEq<[u8]> for Username<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        percent_encoded_equality(self.username.as_bytes(), other, true)
    }
}

impl<'username> PartialEq<Username<'username>> for [u8] {
    fn eq(&self, other: &Username<'username>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a [u8]> for Username<'_> {
    fn eq(&self, other: &&'a [u8]) -> bool {
        self == *other
    }
}

impl<'a, 'username> PartialEq<Username<'username>> for &'a [u8] {
    fn eq(&self, other: &Username<'username>) -> bool {
        other == *self
    }
}

impl PartialEq<str> for Username<'_> {
    fn eq(&self, other: &str) -> bool {
        self == other.as_bytes()
    }
}

impl<'username> PartialEq<Username<'username>> for str {
    fn eq(&self, other: &Username<'username>) -> bool {
        other == self.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for Username<'_> {
    fn eq(&self, other: &&'a str) -> bool {
        self == other.as_bytes()
    }
}

impl<'a, 'username> PartialEq<Username<'username>> for &'a str {
    fn eq(&self, other: &Username<'username>) -> bool {
        other == self.as_bytes()
    }
}

impl<'username> TryFrom<&'username [u8]> for Username<'username> {
    type Error = UsernameError;

    fn try_from(value: &'username [u8]) -> Result<Self, Self::Error> {
        let normalized = check_user_info(value, true)?;

        // Unsafe: The function above [`check_user_info`] ensure this is valid ASCII-US.
        Ok(Username {
            normalized,
            username: Cow::from(unsafe { str::from_utf8_unchecked(value) }),
        })
    }
}

impl<'username> TryFrom<&'username str> for Username<'username> {
    type Error = UsernameError;

    fn try_from(value: &'username str) -> Result<Self, Self::Error> {
        Username::try_from(value.as_bytes())
    }
}

/// An error representing an invalid authority.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AuthorityError {
    /// The host component of the authority was invalid.
    Host(HostError),

    /// The password component of the authority was invalid.
    Password(PasswordError),

    /// The port component of the authority was invalid.
    Port(PortError),

    /// The username component of the authority was invalid.
    Username(UsernameError),
}

impl Display for AuthorityError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::AuthorityError::*;

        match self {
            Host(error) => error.fmt(formatter),
            Password(error) => error.fmt(formatter),
            Port(error) => error.fmt(formatter),
            Username(error) => error.fmt(formatter),
        }
    }
}

impl Error for AuthorityError {}

impl From<Infallible> for AuthorityError {
    fn from(_: Infallible) -> Self {
        AuthorityError::Host(HostError::InvalidIPv4OrRegisteredNameCharacter)
    }
}

impl From<HostError> for AuthorityError {
    fn from(value: HostError) -> Self {
        AuthorityError::Host(value)
    }
}

impl From<PasswordError> for AuthorityError {
    fn from(value: PasswordError) -> Self {
        AuthorityError::Password(value)
    }
}

impl From<PortError> for AuthorityError {
    fn from(value: PortError) -> Self {
        AuthorityError::Port(value)
    }
}

impl From<UserInfoError> for AuthorityError {
    fn from(value: UserInfoError) -> Self {
        use self::AuthorityError::*;

        match value {
            UserInfoError::Password(error) => Password(error),
            UserInfoError::Username(error) => Username(error),
        }
    }
}

impl From<UsernameError> for AuthorityError {
    fn from(value: UsernameError) -> Self {
        AuthorityError::Username(value)
    }
}

/// An error representing an invalid host.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HostError {
    /// The syntax for a future IP literal was used and is not currently supported.
    AddressMechanismNotSupported,

    /// An invalid character for an IPv4 address or registered name was used. Due to the ambiguity
    /// of the grammar, it is not possible to say which. It is also possible that all the characters
    /// were valid, but there was an invalid percent encoding (e.g. `"%ZZ"`).
    InvalidIPv4OrRegisteredNameCharacter,

    /// The syntax for an IPv6 literal was used (i.e. `"[...]"`), but it contained an invalid IPv6
    /// character.
    InvalidIPv6Character,

    /// The syntax for an IPv6 literal was used (i.e. `"[...]"`) and all of the characters were
    /// valid IPv6 characters. However, the format of the literal was invalid.
    InvalidIPv6Format,

    /// The syntax for a future IP literal was used (i.e. `"[v*...]"` where `"*"` is a hexadecimal
    /// digit), but it contained an invalid character.
    InvalidIPvFutureCharacter,
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::HostError::*;

        match self {
            AddressMechanismNotSupported => {
                write!(formatter, "host address mechanism not supported")
            }
            InvalidIPv4OrRegisteredNameCharacter => {
                write!(formatter, "invalid host IPv4 or registered name character")
            }
            InvalidIPv6Character => write!(formatter, "invalid host IPv6 character"),
            InvalidIPv6Format => write!(formatter, "invalid host IPv6 format"),
            InvalidIPvFutureCharacter => write!(formatter, "invalid host IPvFuture character"),
        }
    }
}

impl Error for HostError {}

impl From<Infallible> for HostError {
    fn from(_: Infallible) -> Self {
        HostError::InvalidIPv4OrRegisteredNameCharacter
    }
}

/// An error representing an invalid password component.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PasswordError {
    /// The password contained an invalid character.
    InvalidCharacter,

    /// The password contained an invalid percent encoding (e.g. `"%ZZ"`).
    InvalidPercentEncoding,
}

impl Display for PasswordError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::PasswordError::*;

        match self {
            InvalidCharacter => write!(formatter, "invalid password character"),
            InvalidPercentEncoding => write!(formatter, "invalid password percent encoding"),
        }
    }
}

impl Error for PasswordError {}

impl From<Infallible> for PasswordError {
    fn from(_: Infallible) -> Self {
        PasswordError::InvalidCharacter
    }
}

impl From<UserInfoError> for PasswordError {
    fn from(value: UserInfoError) -> Self {
        match value {
            UserInfoError::Password(error) => error,
            _ => panic!("unexpected user info error"),
        }
    }
}

/// An error representing an invalid port.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PortError {
    /// An invalid character was used in the port. Only decimal digits are allowed.
    InvalidCharacter,

    /// The port was a valid number, but it was too large to fit in a `u16`.
    Overflow,
}

impl Display for PortError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::PortError::*;

        match self {
            InvalidCharacter => write!(formatter, "invalid port character"),
            Overflow => write!(formatter, "port overflow"),
        }
    }
}

impl Error for PortError {}

impl From<Infallible> for PortError {
    fn from(_: Infallible) -> Self {
        PortError::InvalidCharacter
    }
}

/// An error representing an invalid registered name.
///
/// This implies that the registered name contained an invalid host character or had an invalid
/// percent encoding. This error is not possible from parsing an authority. It can only be returned
/// from directly parsing a registered name.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RegisteredNameError;

impl Display for RegisteredNameError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "invalid registered name")
    }
}

impl Error for RegisteredNameError {}

/// An error representing an invalid user information component.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
enum UserInfoError {
    /// The password component of the user information was invalid.
    Password(PasswordError),

    /// The username component of the user information was invalid.
    Username(UsernameError),
}

impl Display for UserInfoError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::UserInfoError::*;

        match self {
            Password(error) => error.fmt(formatter),
            Username(error) => error.fmt(formatter),
        }
    }
}

impl Error for UserInfoError {}

impl From<Infallible> for UserInfoError {
    fn from(_: Infallible) -> Self {
        UserInfoError::Username(UsernameError::InvalidCharacter)
    }
}

impl From<PasswordError> for UserInfoError {
    fn from(value: PasswordError) -> Self {
        UserInfoError::Password(value)
    }
}

impl From<UsernameError> for UserInfoError {
    fn from(value: UsernameError) -> Self {
        UserInfoError::Username(value)
    }
}

/// An error representing an invalid username component.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum UsernameError {
    /// The username contained a `':'` which is only to be used as a delimiter between the username
    /// and password. This variant can only happen when trying to directly parse a username from a
    /// byte source.
    ContainsColon,

    /// The username contained an invalid character.
    InvalidCharacter,

    /// The username contained an invalid percent encoding (e.g. `"%ZZ"`).
    InvalidPercentEncoding,
}

impl Display for UsernameError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use self::UsernameError::*;

        match self {
            ContainsColon => write!(formatter, "username contains a colon character"),
            InvalidCharacter => write!(formatter, "invalid username character"),
            InvalidPercentEncoding => write!(formatter, "invalid username percent encoding"),
        }
    }
}

impl Error for UsernameError {}

impl From<Infallible> for UsernameError {
    fn from(_: Infallible) -> Self {
        UsernameError::InvalidCharacter
    }
}

impl From<UserInfoError> for UsernameError {
    fn from(value: UserInfoError) -> Self {
        match value {
            UserInfoError::Username(error) => error,
            _ => panic!("unexpected user info error"),
        }
    }
}

/// Returns true if the byte string contains only valid IPv4 or registered name characters. This
/// also ensures that percent encodings are valid.
fn check_ipv4_or_registered_name(value: &[u8]) -> (bool, bool) {
    let mut bytes = value.iter();
    let mut normalized = true;

    while let Some(&byte) = bytes.next() {
        match IPV4_AND_REGISTERED_NAME_CHAR_MAP[byte as usize] {
            0 => return (false, false),
            b'%' => match get_percent_encoded_value(bytes.next().cloned(), bytes.next().cloned()) {
                Ok((hex_value, uppercase)) => {
                    if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                        normalized = false;
                    }
                }
                _ => return (false, false),
            },
            b'A'..=b'Z' => normalized = false,
            _ => (),
        }
    }

    (true, normalized)
}

/// Returns true if the byte string contains only valid IPv6 characters.
fn check_ipv6(value: &[u8]) -> bool {
    for &byte in value {
        if !byte.is_ascii_hexdigit() && byte != b':' && byte != b'.' {
            return false;
        }
    }

    true
}

/// Returns true if the byte string contains only valid future IP literal characters. This also
/// ensures that percent encodings are valid.
fn check_ipvfuture(value: &[u8]) -> bool {
    for &byte in value {
        if let 0 = IPV_FUTURE_CHAR_MAP[byte as usize] {
            return false;
        }
    }

    true
}

/// Checks if the user information component contains valid characters and percent encodings. If so,
/// it will return an `Option<usize>` indicating the separator index for the username and password.
fn check_user_info(value: &[u8], is_username: bool) -> Result<bool, UserInfoError> {
    let mut bytes = value.iter();
    let mut normalized = true;

    while let Some(&byte) = bytes.next() {
        match USER_INFO_CHAR_MAP[byte as usize] {
            0 => {
                return if is_username {
                    Err(UsernameError::InvalidCharacter.into())
                } else {
                    Err(PasswordError::InvalidCharacter.into())
                };
            }
            b'%' => match get_percent_encoded_value(bytes.next().cloned(), bytes.next().cloned()) {
                Ok((hex_value, uppercase)) => {
                    if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                        normalized = false;
                    }
                }
                Err(_) => {
                    return if is_username {
                        Err(UsernameError::InvalidPercentEncoding.into())
                    } else {
                        Err(PasswordError::InvalidPercentEncoding.into())
                    };
                }
            },
            b':' if is_username => return Err(UsernameError::ContainsColon.into()),
            _ => (),
        }
    }

    Ok(normalized)
}

/// Parses the authority from the given byte string.
pub(crate) fn parse_authority(value: &[u8]) -> Result<(Authority, &[u8]), AuthorityError> {
    let mut at_index = None;
    let mut last_colon_index = None;
    let mut end_index = value.len();

    for (index, &byte) in value.iter().enumerate() {
        match byte {
            b'@' => {
                if at_index.is_none() {
                    at_index = Some(index);
                    last_colon_index = None;
                }
            }
            b':' => last_colon_index = Some(index),
            b']' => last_colon_index = None,
            b'/' | b'?' | b'#' => {
                end_index = index;
                break;
            }
            _ => (),
        }
    }

    let (value, rest) = value.split_at(end_index);
    let (username, password, host_start_index) = match at_index {
        Some(index) => {
            let (username, password) = parse_user_info(&value[..index])?;
            (Some(username), password, index + 1)
        }
        None => (None, None, 0),
    };

    let (host, port) = match last_colon_index {
        Some(index) => (
            Host::try_from(&value[host_start_index..index])?,
            parse_port(&value[index + 1..])?,
        ),
        None => (Host::try_from(&value[host_start_index..])?, None),
    };

    let authority = Authority {
        host,
        port,
        password,
        username,
    };

    Ok((authority, rest))
}

/// Parses the port from the given byte string.
pub fn parse_port(value: &[u8]) -> Result<Option<u16>, PortError> {
    if value.is_empty() {
        Ok(None)
    } else {
        let mut port = 0u16;

        for &byte in value {
            if !byte.is_ascii_digit() {
                return Err(PortError::InvalidCharacter);
            }

            port = port.checked_mul(10).ok_or(PortError::Overflow)?;
            port = port
                .checked_add((byte - b'0').into())
                .ok_or(PortError::Overflow)?;
        }

        Ok(Some(port))
    }
}

/// Parses the user information from the given byte string.
fn parse_user_info(value: &[u8]) -> Result<(Username, Option<Password>), UserInfoError> {
    let mut bytes = value.iter().enumerate();
    let mut first_colon_index = None;
    let mut password_normalized = true;
    let mut username_normalized = true;

    while let Some((index, &byte)) = bytes.next() {
        match USER_INFO_CHAR_MAP[byte as usize] {
            0 => {
                return if first_colon_index.is_some() {
                    Err(PasswordError::InvalidCharacter.into())
                } else {
                    Err(UsernameError::InvalidCharacter.into())
                }
            }
            b'%' => match get_percent_encoded_value(
                bytes.next().map(|(_, &byte)| byte),
                bytes.next().map(|(_, &byte)| byte),
            ) {
                Ok((hex_value, uppercase)) => {
                    if !uppercase || UNRESERVED_CHAR_MAP[hex_value as usize] != 0 {
                        if first_colon_index.is_some() {
                            password_normalized = false;
                        } else {
                            username_normalized = false;
                        }
                    }
                }
                Err(_) => {
                    return if first_colon_index.is_some() {
                        Err(PasswordError::InvalidPercentEncoding.into())
                    } else {
                        Err(UsernameError::InvalidPercentEncoding.into())
                    }
                }
            },
            b':' => {
                if first_colon_index.is_none() {
                    first_colon_index = Some(index);
                }
            }
            _ => (),
        }
    }

    // Unsafe: All characters are ASCII-US, as checked above.
    Ok(match first_colon_index {
        Some(index) => {
            let username = Username {
                normalized: username_normalized,
                username: Cow::from(unsafe { str::from_utf8_unchecked(&value[..index]) }),
            };
            let password = Password {
                normalized: password_normalized,
                password: Cow::from(unsafe { str::from_utf8_unchecked(&value[index + 1..]) }),
            };
            (username, Some(password))
        }
        _ => {
            let username = Username {
                normalized: username_normalized,
                username: Cow::from(unsafe { str::from_utf8_unchecked(value) }),
            };
            (username, None)
        }
    })
}
