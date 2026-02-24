# five8_core

This crate contains core utilities for the `five8` and `five8_const` crates.

It's unlikely you'll need anything from here except `DecodeError`,
`BASE58_ENCODED_32_MAX_LEN`, and `BASE58_ENCODED_64_MAX_LEN`.
For semver purposes, these are the only things that are part of the public API.
The rest is for internal use only and will not follow semver.
