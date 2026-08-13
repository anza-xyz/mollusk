//! Callbacks used to hook into the invocation pipeline.

#[cfg(feature = "invocation-inspect-callback")]
pub mod invocation_inspect;
pub mod invoke_context;
