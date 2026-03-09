//! Hashing with the [blake3] hash function.
//!
//! [blake3]: https://github.com/BLAKE3-team/BLAKE3
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use solana_hash::{Hash, ParseHashError, HASH_BYTES, MAX_BASE58_LEN};

#[derive(Clone, Default)]
#[cfg(all(
    feature = "blake3",
    not(any(target_os = "solana", target_arch = "bpf"))
))]
pub struct Hasher {
    hasher: blake3::Hasher,
}

#[cfg(all(
    feature = "blake3",
    not(any(target_os = "solana", target_arch = "bpf"))
))]
impl Hasher {
    pub fn hash(&mut self, val: &[u8]) {
        self.hasher.update(val);
    }
    pub fn hashv(&mut self, vals: &[&[u8]]) {
        for val in vals {
            self.hash(val);
        }
    }
    pub fn result(self) -> Hash {
        Hash::new_from_array(*self.hasher.finalize().as_bytes())
    }
}

/// Return a Blake3 hash for the given data.
#[cfg_attr(any(target_os = "solana", target_arch = "bpf"), inline(always))]
pub fn hashv(vals: &[&[u8]]) -> Hash {
    // Perform the calculation inline, calling this from within a program is
    // not supported
    #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
    {
        #[cfg(feature = "blake3")]
        {
            let mut hasher = Hasher::default();
            hasher.hashv(vals);
            hasher.result()
        }
        #[cfg(not(feature = "blake3"))]
        {
            core::hint::black_box(vals);
            panic!("hashv is only available on target `solana` or with the `blake3` feature enabled on this crate")
        }
    }
    // Call via a system call to perform the calculation
    #[cfg(any(target_os = "solana", target_arch = "bpf"))]
    {
        let mut hash_result = core::mem::MaybeUninit::<[u8; solana_hash::HASH_BYTES]>::uninit();
        // SAFETY: This is sound as sol_blake3 always fills all 32 bytes of our hash
        unsafe {
            solana_define_syscall::definitions::sol_blake3(
                vals as *const _ as *const u8,
                vals.len() as u64,
                hash_result.as_mut_ptr() as *mut u8,
            );
            Hash::new_from_array(hash_result.assume_init())
        }
    }
}

/// Return a Blake3 hash for the given data.
#[cfg_attr(any(target_os = "solana", target_arch = "bpf"), inline(always))]
pub fn hash(val: &[u8]) -> Hash {
    hashv(&[val])
}

#[cfg(test)]
#[cfg(feature = "blake3")]
mod tests {
    use super::*;

    #[test]
    fn test_hashv() {
        let val = "gHiljKpq";
        let val_hash = hash(val.as_bytes());

        let ext = "lM890t";
        let ext_hash = hashv(&[val_hash.as_bytes(), ext.as_bytes()]);

        let hash_ext = [&val_hash.to_bytes(), ext.as_bytes()].concat();
        assert!(ext_hash == hash(&hash_ext));
    }
}
