//! Compare system for comparing two instruction results.

use {
    crate::{
        config::{compare, Config},
        types::InstructionResult,
    },
    solana_account::ReadableAccount,
    solana_pubkey::Pubkey,
};

/// Checks to run between two `InstructionResult` instances.
///
/// Similar to `Check`, this allows a developer to dictate the type of checks
/// to run on two results. This is useful for comparing the results of two
/// instructions, or for comparing the result of an instruction against a
/// fixture.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum Compare {
    /// Validate compute units consumed.
    ComputeUnits,
    /// Validate execution time.
    ExecutionTime,
    /// Validate the program result.
    ProgramResult,
    /// Validate the return data.
    ReturnData,
    /// Validate all resulting accounts.
    AllResultingAccounts {
        /// Whether or not to validate each account's data.
        data: bool,
        /// Whether or not to validate each account's executable.
        executable: bool,
        /// Whether or not to validate each account's lamports.
        lamports: bool,
        /// Whether or not to validate each account's owner.
        owner: bool,
        /// Whether or not to validate each account's space.
        space: bool,
    },
    /// Validate the resulting accounts at certain addresses.
    OnlyResultingAccounts {
        /// The addresses on which to apply the validation.
        addresses: Vec<Pubkey>,
        /// Whether or not to validate each account's data.
        data: bool,
        /// Whether or not to validate each account's executable.
        executable: bool,
        /// Whether or not to validate each account's lamports.
        lamports: bool,
        /// Whether or not to validate each account's owner.
        owner: bool,
        /// Whether or not to validate each account's space.
        space: bool,
    },
    /// Validate all of the resulting accounts _except_ the provided addresses.
    AllResultingAccountsExcept {
        /// The addresses on which to _not_ apply the validation.
        ignore_addresses: Vec<Pubkey>,
        /// On non-ignored accounts, whether or not to validate each account's
        /// data.
        data: bool,
        /// On non-ignored accounts, whether or not to validate each account's
        /// executable.
        executable: bool,
        /// On non-ignored accounts, whether or not to validate each account's
        /// lamports.
        lamports: bool,
        /// On non-ignored accounts, whether or not to validate each account's
        /// owner.
        owner: bool,
        /// On non-ignored accounts, whether or not to validate each account's
        /// space.
        space: bool,
    },
}

impl Compare {
    /// Validate all possible checks for all resulting accounts.
    ///
    /// Note: To omit certain checks, use the variant directly, ie.
    /// `Compare::AllResultingAccounts { data: false, .. }`.
    pub fn all_resulting_accounts() -> Self {
        Self::AllResultingAccounts {
            data: true,
            executable: true,
            lamports: true,
            owner: true,
            space: true,
        }
    }

    /// Validate all possible checks for only the resulting accounts at certain
    /// addresses.
    ///
    /// Note: To omit certain checks, use the variant directly, ie.
    /// `Compare::OnlyResultingAccounts { data: false, .. }`.
    pub fn only_resulting_accounts(addresses: &[Pubkey]) -> Self {
        Self::OnlyResultingAccounts {
            addresses: addresses.to_vec(),
            data: true,
            executable: true,
            lamports: true,
            owner: true,
            space: true,
        }
    }

    /// Validate all possible checks for all of the resulting accounts _except_
    /// the provided addresses.
    ///
    /// Note: To omit certain checks, use the variant directly, ie.
    /// `Compare::AllResultingAccountsExcept { data: false, .. }`.
    pub fn all_resulting_accounts_except(ignore_addresses: &[Pubkey]) -> Self {
        Self::AllResultingAccountsExcept {
            ignore_addresses: ignore_addresses.to_vec(),
            data: true,
            executable: true,
            lamports: true,
            owner: true,
            space: true,
        }
    }

    /// Validate everything but compute unit consumption.
    pub fn everything_but_cus() -> Vec<Self> {
        vec![
            // Self::ExecutionTime, // TODO: Intentionally omitted for now...
            Self::ProgramResult,
            Self::ReturnData,
            Self::all_resulting_accounts(),
        ]
    }

    /// Validate everything.
    pub fn everything() -> Vec<Self> {
        vec![
            Self::ComputeUnits,
            // Self::ExecutionTime, // TODO: Intentionally omitted for now...
            Self::ProgramResult,
            Self::ReturnData,
            Self::all_resulting_accounts(),
        ]
    }
}

struct CompareAccountFields {
    data: bool,
    executable: bool,
    lamports: bool,
    owner: bool,
    space: bool,
}

impl InstructionResult {
    fn compare_resulting_accounts(
        &self,
        b: &Self,
        addresses: &[Pubkey],
        ignore_addresses: &[Pubkey],
        fields: CompareAccountFields,
        config: &Config,
    ) -> bool {
        let c = config;
        let mut pass = true;
        for (a, b) in self
            .resulting_accounts
            .iter()
            .zip(b.resulting_accounts.iter())
        {
            if addresses.contains(&a.0) && !ignore_addresses.contains(&a.0) {
                if fields.data {
                    pass &= compare!(c, "resulting_account_data", a.1.data(), b.1.data());
                }
                if fields.executable {
                    pass &= compare!(
                        c,
                        "resulting_account_executable",
                        a.1.executable(),
                        b.1.executable()
                    );
                }
                if fields.lamports {
                    pass &= compare!(
                        c,
                        "resulting_account_lamports",
                        a.1.lamports(),
                        b.1.lamports()
                    );
                }
                if fields.owner {
                    pass &= compare!(c, "resulting_account_owner", a.1.owner(), b.1.owner());
                }
                if fields.space {
                    pass &= compare!(
                        c,
                        "resulting_account_space",
                        a.1.data().len(),
                        b.1.data().len()
                    );
                }
            }
        }
        pass
    }

    /// Compare an `InstructionResult` against another `InstructionResult`.
    pub fn compare_with_config(&self, b: &Self, checks: &[Compare], config: &Config) -> bool {
        let c = config;
        let mut pass = true;
        for check in checks {
            match check {
                Compare::ComputeUnits => {
                    pass &= compare!(
                        c,
                        "compute_units_consumed",
                        self.compute_units_consumed,
                        b.compute_units_consumed
                    );
                }
                Compare::ExecutionTime => {
                    pass &= compare!(c, "execution_time", self.execution_time, b.execution_time);
                }
                Compare::ProgramResult => {
                    pass &= compare!(c, "program_result", self.program_result, b.program_result);
                }
                Compare::ReturnData => {
                    pass &= compare!(c, "return_data", self.return_data, b.return_data);
                }
                Compare::AllResultingAccounts {
                    data,
                    executable,
                    lamports,
                    owner,
                    space,
                } => {
                    pass &= compare!(
                        c,
                        "resulting_accounts_length",
                        self.resulting_accounts.len(),
                        b.resulting_accounts.len()
                    );
                    let addresses = self
                        .resulting_accounts
                        .iter()
                        .map(|(k, _)| *k)
                        .collect::<Vec<_>>();
                    pass &= self.compare_resulting_accounts(
                        b,
                        &addresses,
                        &[],
                        CompareAccountFields {
                            data: *data,
                            executable: *executable,
                            lamports: *lamports,
                            owner: *owner,
                            space: *space,
                        },
                        c,
                    );
                }
                Compare::OnlyResultingAccounts {
                    addresses,
                    data,
                    executable,
                    lamports,
                    owner,
                    space,
                } => {
                    pass &= self.compare_resulting_accounts(
                        b,
                        addresses,
                        &[],
                        CompareAccountFields {
                            data: *data,
                            executable: *executable,
                            lamports: *lamports,
                            owner: *owner,
                            space: *space,
                        },
                        c,
                    );
                }
                Compare::AllResultingAccountsExcept {
                    ignore_addresses,
                    data,
                    executable,
                    lamports,
                    owner,
                    space,
                } => {
                    let addresses = self
                        .resulting_accounts
                        .iter()
                        .map(|(k, _)| *k)
                        .collect::<Vec<_>>();
                    pass &= self.compare_resulting_accounts(
                        b,
                        &addresses,
                        ignore_addresses,
                        CompareAccountFields {
                            data: *data,
                            executable: *executable,
                            lamports: *lamports,
                            owner: *owner,
                            space: *space,
                        },
                        c,
                    );
                }
            }
        }
        pass
    }

    /// Compare an `InstructionResult` against another `InstructionResult`,
    /// panicking on any mismatches.
    pub fn compare(&self, b: &Self) {
        self.compare_with_config(
            b,
            &Compare::everything(),
            &Config {
                panic: true,
                verbose: true,
            },
        );
    }
}
