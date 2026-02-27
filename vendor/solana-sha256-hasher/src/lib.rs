#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(any(target_os = "solana", target_arch = "bpf"))]
pub use solana_define_syscall::definitions::sol_sha256;
use solana_hash::Hash;
#[cfg(any(target_os = "solana", target_arch = "bpf"))]
use {core::mem::MaybeUninit, solana_hash::HASH_BYTES};
#[cfg(all(feature = "sha2", not(any(target_os = "solana", target_arch = "bpf"))))]
use {
    sha2::{Digest, Sha256},
    solana_hash::HASH_BYTES,
};

#[cfg(all(feature = "sha2", not(any(target_os = "solana", target_arch = "bpf"))))]
#[derive(Clone, Default)]
pub struct Hasher {
    hasher: Sha256,
}

#[cfg(all(feature = "sha2", not(any(target_os = "solana", target_arch = "bpf"))))]
impl Hasher {
    #[inline(always)]
    pub fn hash(&mut self, val: &[u8]) {
        self.hasher.update(val);
    }

    #[inline(always)]
    pub fn hashv(&mut self, vals: &[&[u8]]) {
        for val in vals {
            self.hash(val);
        }
    }

    #[inline(always)]
    pub fn result(self) -> Hash {
        let bytes: [u8; HASH_BYTES] = self.hasher.finalize().into();
        bytes.into()
    }
}

/// Return a Sha256 hash for the given data.
#[cfg_attr(target_os = "solana", inline(always))]
pub fn hashv(vals: &[&[u8]]) -> Hash {
    // Perform the calculation inline, calling this from within a program is
    // not supported
    #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
    {
        #[cfg(feature = "sha2")]
        {
            let mut hasher = Hasher::default();
            hasher.hashv(vals);
            hasher.result()
        }
        #[cfg(not(feature = "sha2"))]
        {
            core::hint::black_box(vals);
            panic!("hashv is only available on target `solana` or with the `sha2` feature enabled on this crate")
        }
    }
    // Call via a system call to perform the calculation
    #[cfg(any(target_os = "solana", target_arch = "bpf"))]
    {
        let mut hash_result = MaybeUninit::<[u8; HASH_BYTES]>::uninit();
        // SAFETY: This is sound as sol_sha256 always fills all 32 bytes of our hash
        unsafe {
            sol_sha256(
                vals as *const _ as *const u8,
                vals.len() as u64,
                hash_result.as_mut_ptr() as *mut u8,
            );
            Hash::new_from_array(hash_result.assume_init())
        }
    }
}

/// Return a Sha256 hash for the given data.
#[cfg_attr(target_os = "solana", inline(always))]
pub fn hash(val: &[u8]) -> Hash {
    hashv(&[val])
}
