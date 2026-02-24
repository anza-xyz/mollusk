#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
//! Access to special accounts with dynamically-updated data.
//!
//! Sysvars are special accounts that contain dynamically-updated data about the
//! network cluster, the blockchain history, and the executing transaction. Each
//! sysvar is defined in its own submodule within this module. The [`clock`],
//! [`epoch_schedule`], and [`rent`] sysvars are most useful to on-chain
//! programs.
//!
//! Simple sysvars implement the [`Sysvar::get`] method, which loads a sysvar
//! directly from the runtime, as in this example that logs the `clock` sysvar:
//!
//! ```
//! use solana_account_info::AccountInfo;
//! use solana_msg::msg;
//! use solana_sysvar::Sysvar;
//! use solana_program_error::ProgramResult;
//! use solana_pubkey::Pubkey;
//!
//! fn process_instruction(
//!     program_id: &Pubkey,
//!     accounts: &[AccountInfo],
//!     instruction_data: &[u8],
//! ) -> ProgramResult {
//!     let clock = solana_clock::Clock::get()?;
//!     msg!("clock: {:#?}", clock);
//!     Ok(())
//! }
//! ```
//!
//! Since Solana sysvars are accounts, if the `AccountInfo` is provided to the
//! program, then the program can deserialize the sysvar with
//! [`SysvarSerialize::from_account_info`] to access its data, as in this example that
//! again logs the [`clock`] sysvar.
//!
//! ```
//! use solana_account_info::{AccountInfo, next_account_info};
//! use solana_msg::msg;
//! use solana_sysvar::{Sysvar, SysvarSerialize};
//! use solana_program_error::ProgramResult;
//! use solana_pubkey::Pubkey;
//!
//! fn process_instruction(
//!     program_id: &Pubkey,
//!     accounts: &[AccountInfo],
//!     instruction_data: &[u8],
//! ) -> ProgramResult {
//!     let account_info_iter = &mut accounts.iter();
//!     let clock_account = next_account_info(account_info_iter)?;
//!     let clock = solana_clock::Clock::from_account_info(&clock_account)?;
//!     msg!("clock: {:#?}", clock);
//!     Ok(())
//! }
//! ```
//!
//! When possible, programs should prefer to call `Sysvar::get` instead of
//! deserializing with `Sysvar::from_account_info`, as the latter imposes extra
//! overhead of deserialization while also requiring the sysvar account address
//! be passed to the program, wasting the limited space available to
//! transactions. Deserializing sysvars that can instead be retrieved with
//! `Sysvar::get` should be only be considered for compatibility with older
//! programs that pass around sysvar accounts.
//!
//! Some sysvars are too large to deserialize within a program, and
//! `Sysvar::from_account_info` returns an error, or the serialization attempt
//! will exhaust the program's compute budget. Some sysvars do not implement
//! `Sysvar::get` and return an error. Some sysvars have custom deserializers
//! that do not implement the `Sysvar` trait. These cases are documented in the
//! modules for individual sysvars.
//!
//! All sysvar accounts are owned by the account identified by [`sysvar::ID`].
//!
//! [`sysvar::ID`]: https://docs.rs/solana-sdk-ids/latest/solana_sdk_ids/sysvar/constant.ID.html
//!
//! For more details see the Solana [documentation on sysvars][sysvardoc].
//!
//! [sysvardoc]: https://docs.solanalabs.com/runtime/sysvars

// hidden re-exports to make macros work
pub mod __private {
    #[cfg(target_os = "solana")]
    pub use solana_define_syscall::definitions;
    pub use {solana_program_entrypoint::SUCCESS, solana_program_error::ProgramError};
}
#[cfg(feature = "bincode")]
use {solana_account_info::AccountInfo, solana_sysvar_id::SysvarId};
use {solana_program_error::ProgramError, solana_pubkey::Pubkey};

pub mod clock;
pub mod epoch_rewards;
pub mod epoch_schedule;
pub mod fees;
pub mod last_restart_slot;
pub mod program_stubs;
pub mod recent_blockhashes;
pub mod rent;
pub mod rewards;
pub mod slot_hashes;
pub mod slot_history;

/// Return value indicating that the  `offset + length` is greater than the length of
/// the sysvar data.
//
// Defined in the bpf loader as [`OFFSET_LENGTH_EXCEEDS_SYSVAR`](https://github.com/anza-xyz/agave/blob/master/programs/bpf_loader/src/syscalls/sysvar.rs#L172).
const OFFSET_LENGTH_EXCEEDS_SYSVAR: u64 = 1;

/// Return value indicating that the sysvar was not found.
//
// Defined in the bpf loader as [`SYSVAR_NOT_FOUND`](https://github.com/anza-xyz/agave/blob/master/programs/bpf_loader/src/syscalls/sysvar.rs#L171).
const SYSVAR_NOT_FOUND: u64 = 2;

/// Interface for loading a sysvar.
pub trait Sysvar: Default + Sized {
    /// Load the sysvar directly from the runtime.
    ///
    /// This is the preferred way to load a sysvar. Calling this method does not
    /// incur any deserialization overhead, and does not require the sysvar
    /// account to be passed to the program.
    ///
    /// Not all sysvars support this method. If not, it returns
    /// [`ProgramError::UnsupportedSysvar`].
    fn get() -> Result<Self, ProgramError> {
        Err(ProgramError::UnsupportedSysvar)
    }
}

#[cfg(feature = "bincode")]
/// A type that holds sysvar data.
pub trait SysvarSerialize:
    Sysvar + SysvarId + serde::Serialize + serde::de::DeserializeOwned
{
    /// The size in bytes of the sysvar as serialized account data.
    fn size_of() -> usize {
        bincode::serialized_size(&Self::default()).unwrap() as usize
    }

    /// Deserializes the sysvar from its `AccountInfo`.
    ///
    /// # Errors
    ///
    /// If `account_info` does not have the same ID as the sysvar this function
    /// returns [`ProgramError::InvalidArgument`].
    fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
        if !Self::check_id(account_info.unsigned_key()) {
            return Err(ProgramError::InvalidArgument);
        }
        bincode::deserialize(&account_info.data.borrow()).map_err(|_| ProgramError::InvalidArgument)
    }

    /// Serializes the sysvar to `AccountInfo`.
    ///
    /// # Errors
    ///
    /// Returns `None` if serialization failed.
    fn to_account_info(&self, account_info: &mut AccountInfo) -> Option<()> {
        bincode::serialize_into(&mut account_info.data.borrow_mut()[..], self).ok()
    }
}

/// Implements the [`Sysvar::get`] method for both SBF and host targets.
#[macro_export]
macro_rules! impl_sysvar_get {
    // DEPRECATED: This variant is only for the deprecated Fees sysvar and should be
    // removed once Fees is no longer in use. It uses the old-style direct syscall
    // approach instead of the new sol_get_sysvar syscall.
    ($syscall_name:ident) => {
        fn get() -> Result<Self, $crate::__private::ProgramError> {
            let mut var = Self::default();
            let var_addr = &mut var as *mut _ as *mut u8;

            #[cfg(target_os = "solana")]
            let result = unsafe { $crate::__private::definitions::$syscall_name(var_addr) };

            #[cfg(not(target_os = "solana"))]
            let result = $crate::program_stubs::$syscall_name(var_addr);

            match result {
                $crate::__private::SUCCESS => Ok(var),
                // Unexpected errors are folded into `UnsupportedSysvar`.
                _ => Err($crate::__private::ProgramError::UnsupportedSysvar),
            }
        }
    };
    ($sysvar_id:expr) => {
        fn get() -> Result<Self, $crate::__private::ProgramError> {
            // Allocate uninitialized memory for the sysvar struct
            let mut uninit = core::mem::MaybeUninit::<Self>::uninit();
            let size = core::mem::size_of::<Self>() as u64;
            // Safety: we build a mutable slice pointing to the uninitialized
            // buffer.  The `get_sysvar` syscall will fill exactly `size`
            // bytes, after which the buffer is fully initialised.
            let dst = unsafe {
                core::slice::from_raw_parts_mut(uninit.as_mut_ptr() as *mut u8, size as usize)
            };
            // Attempt to load the sysvar data using the provided sysvar id.
            $crate::get_sysvar(dst, &$sysvar_id, 0, size)?;
            // Safety: `get_sysvar` succeeded and initialised the buffer.
            let var = unsafe { uninit.assume_init() };
            Ok(var)
        }
    };
}

/// Handler for retrieving a slice of sysvar data from the `sol_get_sysvar`
/// syscall.
pub fn get_sysvar(
    dst: &mut [u8],
    sysvar_id: &Pubkey,
    offset: u64,
    length: u64,
) -> Result<(), solana_program_error::ProgramError> {
    // Check that the provided destination buffer is large enough to hold the
    // requested data.
    if dst.len() < length as usize {
        return Err(solana_program_error::ProgramError::InvalidArgument);
    }

    let sysvar_id = sysvar_id as *const _ as *const u8;
    let var_addr = dst as *mut _ as *mut u8;

    #[cfg(target_os = "solana")]
    let result = unsafe {
        solana_define_syscall::definitions::sol_get_sysvar(sysvar_id, var_addr, offset, length)
    };

    #[cfg(not(target_os = "solana"))]
    let result = crate::program_stubs::sol_get_sysvar(sysvar_id, var_addr, offset, length);

    match result {
        solana_program_entrypoint::SUCCESS => Ok(()),
        OFFSET_LENGTH_EXCEEDS_SYSVAR => Err(solana_program_error::ProgramError::InvalidArgument),
        SYSVAR_NOT_FOUND => Err(solana_program_error::ProgramError::UnsupportedSysvar),
        // Unexpected errors are folded into `UnsupportedSysvar`.
        _ => Err(solana_program_error::ProgramError::UnsupportedSysvar),
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::program_stubs::{set_syscall_stubs, SyscallStubs},
        serde_derive::{Deserialize, Serialize},
        solana_program_entrypoint::SUCCESS,
        solana_program_error::ProgramError,
        solana_pubkey::Pubkey,
        std::{cell::RefCell, rc::Rc},
    };

    #[repr(C)]
    #[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
    struct TestSysvar {
        something: Pubkey,
    }
    solana_pubkey::declare_id!("TestSysvar111111111111111111111111111111111");
    impl solana_sysvar_id::SysvarId for TestSysvar {
        fn id() -> solana_pubkey::Pubkey {
            id()
        }

        fn check_id(pubkey: &solana_pubkey::Pubkey) -> bool {
            check_id(pubkey)
        }
    }
    impl Sysvar for TestSysvar {}
    impl SysvarSerialize for TestSysvar {}

    // NOTE tests that use this mock MUST carry the #[serial] attribute
    struct MockGetSysvarSyscall {
        data: Vec<u8>,
    }
    impl SyscallStubs for MockGetSysvarSyscall {
        #[allow(clippy::arithmetic_side_effects)]
        fn sol_get_sysvar(
            &self,
            _sysvar_id_addr: *const u8,
            var_addr: *mut u8,
            offset: u64,
            length: u64,
        ) -> u64 {
            let slice = unsafe { std::slice::from_raw_parts_mut(var_addr, length as usize) };
            slice.copy_from_slice(&self.data[offset as usize..(offset + length) as usize]);
            SUCCESS
        }
    }
    pub fn mock_get_sysvar_syscall(data: &[u8]) {
        set_syscall_stubs(Box::new(MockGetSysvarSyscall {
            data: data.to_vec(),
        }));
    }

    /// Convert a value to its in-memory byte representation.
    ///
    /// Safety: This relies on the type's plain old data layout. Intended for tests.
    pub fn to_bytes<T>(value: &T) -> Vec<u8> {
        unsafe {
            let size = core::mem::size_of::<T>();
            let ptr = (value as *const T) as *const u8;
            let mut data = vec![0u8; size];
            std::ptr::copy_nonoverlapping(ptr, data.as_mut_ptr(), size);
            data
        }
    }

    #[test]
    fn test_sysvar_account_info_to_from() {
        let test_sysvar = TestSysvar::default();
        let key = id();
        let wrong_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut lamports = 42;
        let mut data = vec![0_u8; TestSysvar::size_of()];
        let mut account_info =
            AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);

        test_sysvar.to_account_info(&mut account_info).unwrap();
        let new_test_sysvar = TestSysvar::from_account_info(&account_info).unwrap();
        assert_eq!(test_sysvar, new_test_sysvar);

        account_info.key = &wrong_key;
        assert_eq!(
            TestSysvar::from_account_info(&account_info),
            Err(ProgramError::InvalidArgument)
        );

        let mut small_data = vec![];
        account_info.data = Rc::new(RefCell::new(&mut small_data));
        assert_eq!(test_sysvar.to_account_info(&mut account_info), None);
    }
}
