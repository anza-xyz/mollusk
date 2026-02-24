mod utility;

pub mod authority;
pub mod fragment;
pub mod path;
pub mod query;
pub mod relative_reference;
pub mod scheme;
pub mod uri;
pub mod uri_reference;

pub use self::authority::{
    Authority, AuthorityError, Host, HostError, Password, PasswordError, PortError, RegisteredName,
    RegisteredNameError, Username, UsernameError,
};
pub use self::fragment::{Fragment, FragmentError};
pub use self::path::{Path, PathError, Segment};
pub use self::query::{Query, QueryError};
pub use self::relative_reference::{
    RelativeReference, RelativeReferenceBuilder, RelativeReferenceError,
};
pub use self::scheme::{Scheme, SchemeError, SchemeStatus, UnregisteredScheme};
pub use self::uri::{URIBuilder, URIError, URI};
pub use self::uri_reference::{URIReference, URIReferenceBuilder, URIReferenceError};
