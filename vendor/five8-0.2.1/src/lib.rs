#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![no_std]
#[cfg(feature = "std")]
extern crate std;
#[cfg(target_feature = "avx2")]
mod avx;

#[cfg(target_feature = "avx2")]
mod bits_find_lsb;

mod decode;
mod encode;
pub use decode::{decode_32, decode_64};
pub use encode::{encode_32, encode_64};
pub use five8_core::{DecodeError, BASE58_ENCODED_32_MAX_LEN, BASE58_ENCODED_64_MAX_LEN};
#[cfg(feature = "dev-utils")]
pub use {
    decode::{truncate_and_swap_u64s_64_pub, truncate_and_swap_u64s_scalar_pub},
    encode::{
        in_leading_0s_32_pub, in_leading_0s_scalar_pub, intermediate_to_base58_32_pub,
        intermediate_to_base58_scalar_64_pub, make_binary_array_32_pub, make_binary_array_64_pub,
        make_intermediate_array_32_pub, make_intermediate_array_64_pub,
    },
};
mod unlikely;
