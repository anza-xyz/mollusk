#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
#[cfg(feature = "num-traits")]
use num_traits::ToPrimitive;
#[cfg(feature = "frozen-abi")]
extern crate std;
use {core::fmt, solana_program_error::ProgramError};
pub use {
    instruction_error_module::*,
    solana_program_error::{
        ACCOUNT_ALREADY_INITIALIZED, ACCOUNT_BORROW_FAILED, ACCOUNT_DATA_TOO_SMALL,
        ACCOUNT_NOT_RENT_EXEMPT, ARITHMETIC_OVERFLOW, BORSH_IO_ERROR,
        BUILTIN_PROGRAMS_MUST_CONSUME_COMPUTE_UNITS, CUSTOM_ZERO, ILLEGAL_OWNER, IMMUTABLE,
        INCORRECT_AUTHORITY, INCORRECT_PROGRAM_ID, INSUFFICIENT_FUNDS, INVALID_ACCOUNT_DATA,
        INVALID_ACCOUNT_DATA_REALLOC, INVALID_ACCOUNT_OWNER, INVALID_ARGUMENT,
        INVALID_INSTRUCTION_DATA, INVALID_SEEDS, MAX_ACCOUNTS_DATA_ALLOCATIONS_EXCEEDED,
        MAX_INSTRUCTION_TRACE_LENGTH_EXCEEDED, MAX_SEED_LENGTH_EXCEEDED,
        MISSING_REQUIRED_SIGNATURES, NOT_ENOUGH_ACCOUNT_KEYS, UNINITIALIZED_ACCOUNT,
        UNSUPPORTED_SYSVAR,
    },
};

#[allow(deprecated)]
mod instruction_error_module {
    #[cfg(feature = "frozen-abi")]
    use solana_frozen_abi_macro::{AbiEnumVisitor, AbiExample};

    /// Reasons the runtime might have rejected an instruction.
    ///
    /// Members of this enum must not be removed, but new ones can be added.
    /// Also, it is crucial that meta-information if any that comes along with
    /// an error be consistent across software versions.  For example, it is
    /// dangerous to include error strings from 3rd party crates because they could
    /// change at any time and changes to them are difficult to detect.
    #[cfg_attr(feature = "frozen-abi", derive(AbiExample, AbiEnumVisitor))]
    #[cfg_attr(
        feature = "serde",
        derive(serde_derive::Serialize, serde_derive::Deserialize)
    )]
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum InstructionError {
        /// Deprecated! Use CustomError instead!
        /// The program instruction returned an error
        GenericError,

        /// The arguments provided to a program were invalid
        InvalidArgument,

        /// An instruction's data contents were invalid
        InvalidInstructionData,

        /// An account's data contents was invalid
        InvalidAccountData,

        /// An account's data was too small
        AccountDataTooSmall,

        /// An account's balance was too small to complete the instruction
        InsufficientFunds,

        /// The account did not have the expected program id
        IncorrectProgramId,

        /// A signature was required but not found
        MissingRequiredSignature,

        /// An initialize instruction was sent to an account that has already been initialized.
        AccountAlreadyInitialized,

        /// An attempt to operate on an account that hasn't been initialized.
        UninitializedAccount,

        /// Program's instruction lamport balance does not equal the balance after the instruction
        UnbalancedInstruction,

        /// Program illegally modified an account's program id
        ModifiedProgramId,

        /// Program spent the lamports of an account that doesn't belong to it
        ExternalAccountLamportSpend,

        /// Program modified the data of an account that doesn't belong to it
        ExternalAccountDataModified,

        /// Read-only account's lamports modified
        ReadonlyLamportChange,

        /// Read-only account's data was modified
        ReadonlyDataModified,

        /// An account was referenced more than once in a single instruction
        // Deprecated, instructions can now contain duplicate accounts
        DuplicateAccountIndex,

        /// Executable bit on account changed, but shouldn't have
        ExecutableModified,

        /// Rent_epoch account changed, but shouldn't have
        RentEpochModified,

        /// The instruction expected additional account keys
        #[deprecated(since = "2.1.0", note = "Use InstructionError::MissingAccount instead")]
        NotEnoughAccountKeys,

        /// Program other than the account's owner changed the size of the account data
        AccountDataSizeChanged,

        /// The instruction expected an executable account
        AccountNotExecutable,

        /// Failed to borrow a reference to account data, already borrowed
        AccountBorrowFailed,

        /// Account data has an outstanding reference after a program's execution
        AccountBorrowOutstanding,

        /// The same account was multiply passed to an on-chain program's entrypoint, but the program
        /// modified them differently.  A program can only modify one instance of the account because
        /// the runtime cannot determine which changes to pick or how to merge them if both are modified
        DuplicateAccountOutOfSync,

        /// Allows on-chain programs to implement program-specific error types and see them returned
        /// by the Solana runtime. A program-specific error may be any type that is represented as
        /// or serialized to a u32 integer.
        Custom(u32),

        /// The return value from the program was invalid.  Valid errors are either a defined builtin
        /// error value or a user-defined error in the lower 32 bits.
        InvalidError,

        /// Executable account's data was modified
        ExecutableDataModified,

        /// Executable account's lamports modified
        ExecutableLamportChange,

        /// Executable accounts must be rent exempt
        ExecutableAccountNotRentExempt,

        /// Unsupported program id
        UnsupportedProgramId,

        /// Cross-program invocation call depth too deep
        CallDepth,

        /// An account required by the instruction is missing
        MissingAccount,

        /// Cross-program invocation reentrancy not allowed for this instruction
        ReentrancyNotAllowed,

        /// Length of the seed is too long for address generation
        MaxSeedLengthExceeded,

        /// Provided seeds do not result in a valid address
        InvalidSeeds,

        /// Failed to reallocate account data of this length
        InvalidRealloc,

        /// Computational budget exceeded
        ComputationalBudgetExceeded,

        /// Cross-program invocation with unauthorized signer or writable account
        PrivilegeEscalation,

        /// Failed to create program execution environment
        ProgramEnvironmentSetupFailure,

        /// Program failed to complete
        ProgramFailedToComplete,

        /// Program failed to compile
        ProgramFailedToCompile,

        /// Account is immutable
        Immutable,

        /// Incorrect authority provided
        IncorrectAuthority,

        /// Failed to serialize or deserialize account data
        BorshIoError,

        /// An account does not have enough lamports to be rent-exempt
        AccountNotRentExempt,

        /// Invalid account owner
        InvalidAccountOwner,

        /// Program arithmetic overflowed
        ArithmeticOverflow,

        /// Unsupported sysvar
        UnsupportedSysvar,

        /// Illegal account owner
        IllegalOwner,

        /// Accounts data allocations exceeded the maximum allowed per transaction
        MaxAccountsDataAllocationsExceeded,

        /// Max accounts exceeded
        MaxAccountsExceeded,

        /// Max instruction trace length exceeded
        MaxInstructionTraceLengthExceeded,

        /// Builtin programs must consume compute units
        BuiltinProgramsMustConsumeComputeUnits,
        // Note: For any new error added here an equivalent ProgramError and its
        // conversions must also be added
    }
}

impl core::error::Error for InstructionError {}

impl fmt::Display for InstructionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InstructionError::GenericError => f.write_str("generic instruction error"),
            InstructionError::InvalidArgument => f.write_str("invalid program argument"),
            InstructionError::InvalidInstructionData => f.write_str("invalid instruction data"),
            InstructionError::InvalidAccountData => {
                f.write_str("invalid account data for instruction")
            }
            InstructionError::AccountDataTooSmall => {
                f.write_str("account data too small for instruction")
            }
            InstructionError::InsufficientFunds => {
                f.write_str("insufficient funds for instruction")
            }
            InstructionError::IncorrectProgramId => {
                f.write_str("incorrect program id for instruction")
            }
            InstructionError::MissingRequiredSignature => {
                f.write_str("missing required signature for instruction")
            }
            InstructionError::AccountAlreadyInitialized => {
                f.write_str("instruction requires an uninitialized account")
            }
            InstructionError::UninitializedAccount => {
                f.write_str("instruction requires an initialized account")
            }
            InstructionError::UnbalancedInstruction => {
                f.write_str("sum of account balances before and after instruction do not match")
            }
            InstructionError::ModifiedProgramId => {
                f.write_str("instruction illegally modified the program id of an account")
            }
            InstructionError::ExternalAccountLamportSpend => {
                f.write_str("instruction spent from the balance of an account it does not own")
            }
            InstructionError::ExternalAccountDataModified => {
                f.write_str("instruction modified data of an account it does not own")
            }
            InstructionError::ReadonlyLamportChange => {
                f.write_str("instruction changed the balance of a read-only account")
            }
            InstructionError::ReadonlyDataModified => {
                f.write_str("instruction modified data of a read-only account")
            }
            InstructionError::DuplicateAccountIndex => {
                f.write_str("instruction contains duplicate accounts")
            }
            InstructionError::ExecutableModified => {
                f.write_str("instruction changed executable bit of an account")
            }
            InstructionError::RentEpochModified => {
                f.write_str("instruction modified rent epoch of an account")
            }
            #[allow(deprecated)]
            InstructionError::NotEnoughAccountKeys => {
                f.write_str("insufficient account keys for instruction")
            }
            InstructionError::AccountDataSizeChanged => f.write_str(
                "program other than the account's owner changed the size of the account data",
            ),
            InstructionError::AccountNotExecutable => {
                f.write_str("instruction expected an executable account")
            }
            InstructionError::AccountBorrowFailed => f.write_str(
                "instruction tries to borrow reference for an account which is already borrowed",
            ),
            InstructionError::AccountBorrowOutstanding => {
                f.write_str("instruction left account with an outstanding borrowed reference")
            }
            InstructionError::DuplicateAccountOutOfSync => {
                f.write_str("instruction modifications of multiply-passed account differ")
            }
            InstructionError::Custom(num) => {
                write!(f, "custom program error: {num:#x}")
            }
            InstructionError::InvalidError => f.write_str("program returned invalid error code"),
            InstructionError::ExecutableDataModified => {
                f.write_str("instruction changed executable accounts data")
            }
            InstructionError::ExecutableLamportChange => {
                f.write_str("instruction changed the balance of an executable account")
            }
            InstructionError::ExecutableAccountNotRentExempt => {
                f.write_str("executable accounts must be rent exempt")
            }
            InstructionError::UnsupportedProgramId => f.write_str("Unsupported program id"),
            InstructionError::CallDepth => {
                f.write_str("Cross-program invocation call depth too deep")
            }
            InstructionError::MissingAccount => {
                f.write_str("An account required by the instruction is missing")
            }
            InstructionError::ReentrancyNotAllowed => {
                f.write_str("Cross-program invocation reentrancy not allowed for this instruction")
            }
            InstructionError::MaxSeedLengthExceeded => {
                f.write_str("Length of the seed is too long for address generation")
            }
            InstructionError::InvalidSeeds => {
                f.write_str("Provided seeds do not result in a valid address")
            }
            InstructionError::InvalidRealloc => f.write_str("Failed to reallocate account data"),
            InstructionError::ComputationalBudgetExceeded => {
                f.write_str("Computational budget exceeded")
            }
            InstructionError::PrivilegeEscalation => {
                f.write_str("Cross-program invocation with unauthorized signer or writable account")
            }
            InstructionError::ProgramEnvironmentSetupFailure => {
                f.write_str("Failed to create program execution environment")
            }
            InstructionError::ProgramFailedToComplete => f.write_str("Program failed to complete"),
            InstructionError::ProgramFailedToCompile => f.write_str("Program failed to compile"),
            InstructionError::Immutable => f.write_str("Account is immutable"),
            InstructionError::IncorrectAuthority => f.write_str("Incorrect authority provided"),
            InstructionError::BorshIoError => {
                f.write_str("Failed to serialize or deserialize account data")
            }
            InstructionError::AccountNotRentExempt => {
                f.write_str("An account does not have enough lamports to be rent-exempt")
            }
            InstructionError::InvalidAccountOwner => f.write_str("Invalid account owner"),
            InstructionError::ArithmeticOverflow => f.write_str("Program arithmetic overflowed"),
            InstructionError::UnsupportedSysvar => f.write_str("Unsupported sysvar"),
            InstructionError::IllegalOwner => f.write_str("Provided owner is not allowed"),
            InstructionError::MaxAccountsDataAllocationsExceeded => f.write_str(
                "Accounts data allocations exceeded the maximum allowed per transaction",
            ),
            InstructionError::MaxAccountsExceeded => f.write_str("Max accounts exceeded"),
            InstructionError::MaxInstructionTraceLengthExceeded => {
                f.write_str("Max instruction trace length exceeded")
            }
            InstructionError::BuiltinProgramsMustConsumeComputeUnits => {
                f.write_str("Builtin programs must consume compute units")
            }
        }
    }
}

#[cfg(feature = "num-traits")]
impl<T> From<T> for InstructionError
where
    T: ToPrimitive,
{
    fn from(error: T) -> Self {
        let error = error.to_u64().unwrap_or(0xbad_c0de);
        match error {
            CUSTOM_ZERO => Self::Custom(0),
            INVALID_ARGUMENT => Self::InvalidArgument,
            INVALID_INSTRUCTION_DATA => Self::InvalidInstructionData,
            INVALID_ACCOUNT_DATA => Self::InvalidAccountData,
            ACCOUNT_DATA_TOO_SMALL => Self::AccountDataTooSmall,
            INSUFFICIENT_FUNDS => Self::InsufficientFunds,
            INCORRECT_PROGRAM_ID => Self::IncorrectProgramId,
            MISSING_REQUIRED_SIGNATURES => Self::MissingRequiredSignature,
            ACCOUNT_ALREADY_INITIALIZED => Self::AccountAlreadyInitialized,
            UNINITIALIZED_ACCOUNT => Self::UninitializedAccount,
            #[allow(deprecated)]
            NOT_ENOUGH_ACCOUNT_KEYS => Self::NotEnoughAccountKeys,
            ACCOUNT_BORROW_FAILED => Self::AccountBorrowFailed,
            MAX_SEED_LENGTH_EXCEEDED => Self::MaxSeedLengthExceeded,
            INVALID_SEEDS => Self::InvalidSeeds,
            BORSH_IO_ERROR => Self::BorshIoError,
            ACCOUNT_NOT_RENT_EXEMPT => Self::AccountNotRentExempt,
            UNSUPPORTED_SYSVAR => Self::UnsupportedSysvar,
            ILLEGAL_OWNER => Self::IllegalOwner,
            MAX_ACCOUNTS_DATA_ALLOCATIONS_EXCEEDED => Self::MaxAccountsDataAllocationsExceeded,
            INVALID_ACCOUNT_DATA_REALLOC => Self::InvalidRealloc,
            MAX_INSTRUCTION_TRACE_LENGTH_EXCEEDED => Self::MaxInstructionTraceLengthExceeded,
            BUILTIN_PROGRAMS_MUST_CONSUME_COMPUTE_UNITS => {
                Self::BuiltinProgramsMustConsumeComputeUnits
            }
            INVALID_ACCOUNT_OWNER => Self::InvalidAccountOwner,
            ARITHMETIC_OVERFLOW => Self::ArithmeticOverflow,
            IMMUTABLE => Self::Immutable,
            INCORRECT_AUTHORITY => Self::IncorrectAuthority,
            _ => {
                // A valid custom error has no bits set in the upper 32
                if error >> solana_program_error::BUILTIN_BIT_SHIFT == 0 {
                    Self::Custom(error as u32)
                } else {
                    Self::InvalidError
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum LamportsError {
    /// arithmetic underflowed
    ArithmeticUnderflow,
    /// arithmetic overflowed
    ArithmeticOverflow,
}

impl core::error::Error for LamportsError {}

impl fmt::Display for LamportsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ArithmeticUnderflow => f.write_str("Arithmetic underflowed"),
            Self::ArithmeticOverflow => f.write_str("Arithmetic overflowed"),
        }
    }
}

impl From<LamportsError> for InstructionError {
    fn from(error: LamportsError) -> Self {
        match error {
            LamportsError::ArithmeticOverflow => InstructionError::ArithmeticOverflow,
            LamportsError::ArithmeticUnderflow => InstructionError::ArithmeticOverflow,
        }
    }
}

impl TryFrom<InstructionError> for ProgramError {
    type Error = InstructionError;

    fn try_from(error: InstructionError) -> Result<Self, Self::Error> {
        match error {
            Self::Error::Custom(err) => Ok(Self::Custom(err)),
            Self::Error::InvalidArgument => Ok(Self::InvalidArgument),
            Self::Error::InvalidInstructionData => Ok(Self::InvalidInstructionData),
            Self::Error::InvalidAccountData => Ok(Self::InvalidAccountData),
            Self::Error::AccountDataTooSmall => Ok(Self::AccountDataTooSmall),
            Self::Error::InsufficientFunds => Ok(Self::InsufficientFunds),
            Self::Error::IncorrectProgramId => Ok(Self::IncorrectProgramId),
            Self::Error::MissingRequiredSignature => Ok(Self::MissingRequiredSignature),
            Self::Error::AccountAlreadyInitialized => Ok(Self::AccountAlreadyInitialized),
            Self::Error::UninitializedAccount => Ok(Self::UninitializedAccount),
            #[allow(deprecated)]
            Self::Error::NotEnoughAccountKeys => Ok(Self::NotEnoughAccountKeys),
            Self::Error::MissingAccount => Ok(Self::NotEnoughAccountKeys),
            Self::Error::AccountBorrowFailed => Ok(Self::AccountBorrowFailed),
            Self::Error::MaxSeedLengthExceeded => Ok(Self::MaxSeedLengthExceeded),
            Self::Error::InvalidSeeds => Ok(Self::InvalidSeeds),
            Self::Error::BorshIoError => Ok(Self::BorshIoError),
            Self::Error::AccountNotRentExempt => Ok(Self::AccountNotRentExempt),
            Self::Error::UnsupportedSysvar => Ok(Self::UnsupportedSysvar),
            Self::Error::IllegalOwner => Ok(Self::IllegalOwner),
            Self::Error::MaxAccountsDataAllocationsExceeded => {
                Ok(Self::MaxAccountsDataAllocationsExceeded)
            }
            Self::Error::InvalidRealloc => Ok(Self::InvalidRealloc),
            Self::Error::MaxInstructionTraceLengthExceeded => {
                Ok(Self::MaxInstructionTraceLengthExceeded)
            }
            Self::Error::BuiltinProgramsMustConsumeComputeUnits => {
                Ok(Self::BuiltinProgramsMustConsumeComputeUnits)
            }
            Self::Error::InvalidAccountOwner => Ok(Self::InvalidAccountOwner),
            Self::Error::ArithmeticOverflow => Ok(Self::ArithmeticOverflow),
            Self::Error::Immutable => Ok(Self::Immutable),
            Self::Error::IncorrectAuthority => Ok(Self::IncorrectAuthority),
            _ => Err(error),
        }
    }
}
