//! Results of Mollusk program execution.

use {
    solana_account::{Account, ReadableAccount},
    solana_instruction::error::InstructionError,
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
};

macro_rules! compare {
    ($c:expr, $check:expr, $left:expr, $right:expr $(,)?) => {{
        if $left != $right {
            let msg = format!(
                "CHECK FAILED: {}\n  Expected: `{:?}`,\n Got: `{:?}`",
                $check, $left, $right
            );
            if $c.panic {
                panic!("{}", msg);
            } else {
                if $c.verbose {
                    println!("{}", msg);
                }
                return false;
            }
        }
        true
    }};
}

macro_rules! throw {
    ($c:expr, $($arg:tt)+) => {{
        let msg = format!($($arg)+);
        if $c.panic {
            panic!("{}", msg);
        } else {
            if $c.verbose {
                eprintln!("{}", msg);
            }
        }
        false
    }};
}

/// The result code of the program's execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramResult {
    /// The program executed successfully.
    Success,
    /// The program returned an error.
    Failure(ProgramError),
    /// Mollusk encountered an error while executing the program.
    UnknownError(InstructionError),
}

impl ProgramResult {
    /// Returns `true` if the program returned an error.
    pub fn is_err(&self) -> bool {
        !matches!(self, ProgramResult::Success)
    }
}

impl From<Result<(), InstructionError>> for ProgramResult {
    fn from(result: Result<(), InstructionError>) -> Self {
        match result {
            Ok(()) => ProgramResult::Success,
            Err(err) => {
                if let Ok(program_error) = ProgramError::try_from(err.clone()) {
                    ProgramResult::Failure(program_error)
                } else {
                    ProgramResult::UnknownError(err)
                }
            }
        }
    }
}

/// The overall result of the instruction.
#[derive(Debug, PartialEq, Eq)]
pub struct InstructionResult {
    /// The number of compute units consumed by the instruction.
    pub compute_units_consumed: u64,
    /// The time taken to execute the instruction.
    pub execution_time: u64,
    /// The result code of the program's execution.
    pub program_result: ProgramResult,
    /// The raw result of the program's execution.
    pub raw_result: Result<(), InstructionError>,
    /// The return data produced by the instruction, if any.
    pub return_data: Vec<u8>,
    /// The resulting accounts after executing the instruction.
    ///
    /// This includes all accounts provided to the processor, in the order
    /// they were provided. Any accounts that were modified will maintain
    /// their original position in this list, but with updated state.
    pub resulting_accounts: Vec<(Pubkey, Account)>,
}

impl Default for InstructionResult {
    fn default() -> Self {
        Self {
            compute_units_consumed: 0,
            execution_time: 0,
            program_result: ProgramResult::Success,
            raw_result: Ok(()),
            return_data: vec![],
            resulting_accounts: vec![],
        }
    }
}

pub struct Config {
    pub panic: bool,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panic: true,
            verbose: false,
        }
    }
}

impl InstructionResult {
    /// Get an account from the resulting accounts by its pubkey.
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&Account> {
        self.resulting_accounts
            .iter()
            .find(|(k, _)| k == pubkey)
            .map(|(_, a)| a)
    }

    /// Perform checks on the instruction result.
    pub fn run_checks_with_config(&self, checks: &[Check], config: &Config) -> bool {
        let c = config;
        let mut pass = true;
        for check in checks {
            match &check.check {
                CheckType::ComputeUnitsConsumed(units) => {
                    let check_units = *units;
                    let actual_units = self.compute_units_consumed;
                    pass &= compare!(c, "compute_units", check_units, actual_units);
                }
                CheckType::ExecutionTime(time) => {
                    let check_time = *time;
                    let actual_time = self.execution_time;
                    pass &= compare!(c, "execution_time", check_time, actual_time);
                }
                CheckType::ProgramResult(result) => {
                    let check_result = result;
                    let actual_result = &self.program_result;
                    pass &= compare!(c, "program_result", check_result, actual_result);
                }
                CheckType::ReturnData(return_data) => {
                    let check_return_data = return_data;
                    let actual_return_data = &self.return_data;
                    pass &= compare!(c, "return_data", check_return_data, actual_return_data);
                }
                CheckType::ResultingAccount(account) => {
                    let pubkey = account.pubkey;
                    let Some(resulting_account) = self
                        .resulting_accounts
                        .iter()
                        .find(|(k, _)| k == &pubkey)
                        .map(|(_, a)| a)
                    else {
                        pass &= throw!(c, "Account not found in resulting accounts: {}", pubkey);
                        continue;
                    };
                    if let Some(check_data) = account.check_data {
                        let actual_data = resulting_account.data();
                        pass &= compare!(c, "account_data", check_data, actual_data);
                    }
                    if let Some(check_executable) = account.check_executable {
                        let actual_executable = resulting_account.executable();
                        pass &=
                            compare!(c, "account_executable", check_executable, actual_executable);
                    }
                    if let Some(check_lamports) = account.check_lamports {
                        let actual_lamports = resulting_account.lamports();
                        pass &= compare!(c, "account_lamports", check_lamports, actual_lamports);
                    }
                    if let Some(check_owner) = account.check_owner {
                        let actual_owner = resulting_account.owner();
                        pass &= compare!(c, "account_owner", check_owner, actual_owner);
                    }
                    if let Some(check_space) = account.check_space {
                        let actual_space = resulting_account.data().len();
                        pass &= compare!(c, "account_space", check_space, actual_space);
                    }
                    if let Some(check_state) = &account.check_state {
                        match check_state {
                            AccountStateCheck::Closed => {
                                pass &= compare!(
                                    c,
                                    "account_closed",
                                    true,
                                    resulting_account == &Account::default(),
                                );
                            }
                        }
                    }
                    if let Some((offset, check_data_slice)) = account.check_data_slice {
                        let actual_data = resulting_account.data();
                        if offset + check_data_slice.len() > actual_data.len() {
                            pass &= throw!(
                                c,
                                "Account data slice: offset {} + slice length {} exceeds account \
                                 data length {}",
                                offset,
                                check_data_slice.len(),
                                actual_data.len(),
                            );
                            continue;
                        }
                        let actual_data_slice =
                            &actual_data[offset..offset + check_data_slice.len()];
                        pass &=
                            compare!(c, "account_data_slice", check_data_slice, actual_data_slice,);
                    }
                }
            }
        }
        pass
    }

    /// Perform checks on the instruction result, panicking on any mismatches.
    pub fn run_checks(&self, checks: &[Check]) {
        self.run_checks_with_config(
            checks,
            &Config {
                panic: true,
                verbose: true,
            },
        );
    }

    pub(crate) fn absorb(&mut self, other: Self) {
        self.compute_units_consumed += other.compute_units_consumed;
        self.execution_time += other.execution_time;
        self.program_result = other.program_result;
        self.raw_result = other.raw_result;
        self.return_data = other.return_data;
        self.resulting_accounts = other.resulting_accounts;
    }

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

enum CheckType<'a> {
    /// Check the number of compute units consumed by the instruction.
    ComputeUnitsConsumed(u64),
    /// Check the time taken to execute the instruction.
    ExecutionTime(u64),
    /// Check the result code of the program's execution.
    ProgramResult(ProgramResult),
    /// Check the return data produced by executing the instruction.
    ReturnData(&'a [u8]),
    /// Check a resulting account after executing the instruction.
    ResultingAccount(AccountCheck<'a>),
}

pub struct Check<'a> {
    check: CheckType<'a>,
}

impl<'a> Check<'a> {
    fn new(check: CheckType<'a>) -> Self {
        Self { check }
    }

    /// Check the number of compute units consumed by the instruction.
    pub fn compute_units(units: u64) -> Self {
        Check::new(CheckType::ComputeUnitsConsumed(units))
    }

    /// Check the time taken to execute the instruction.
    pub fn time(time: u64) -> Self {
        Check::new(CheckType::ExecutionTime(time))
    }

    /// Assert that the program executed successfully.
    pub fn success() -> Self {
        Check::new(CheckType::ProgramResult(ProgramResult::Success))
    }

    /// Assert that the program returned an error.
    pub fn err(error: ProgramError) -> Self {
        Check::new(CheckType::ProgramResult(ProgramResult::Failure(error)))
    }

    /// Assert that the instruction returned an error.
    pub fn instruction_err(error: InstructionError) -> Self {
        Check::new(CheckType::ProgramResult(ProgramResult::UnknownError(error)))
    }

    /// Assert that the instruction returned the provided result.
    pub fn program_result(result: ProgramResult) -> Self {
        Check::new(CheckType::ProgramResult(result))
    }

    /// Check the return data produced by executing the instruction.
    pub fn return_data(return_data: &'a [u8]) -> Self {
        Check::new(CheckType::ReturnData(return_data))
    }

    /// Check a resulting account after executing the instruction.
    pub fn account(pubkey: &Pubkey) -> AccountCheckBuilder {
        AccountCheckBuilder::new(pubkey)
    }
}

enum AccountStateCheck {
    Closed,
}

struct AccountCheck<'a> {
    pubkey: Pubkey,
    check_data: Option<&'a [u8]>,
    check_executable: Option<bool>,
    check_lamports: Option<u64>,
    check_owner: Option<&'a Pubkey>,
    check_space: Option<usize>,
    check_state: Option<AccountStateCheck>,
    check_data_slice: Option<(usize, &'a [u8])>,
}

impl AccountCheck<'_> {
    fn new(pubkey: &Pubkey) -> Self {
        Self {
            pubkey: *pubkey,
            check_data: None,
            check_executable: None,
            check_lamports: None,
            check_owner: None,
            check_space: None,
            check_state: None,
            check_data_slice: None,
        }
    }
}

pub struct AccountCheckBuilder<'a> {
    check: AccountCheck<'a>,
}

impl<'a> AccountCheckBuilder<'a> {
    fn new(pubkey: &Pubkey) -> Self {
        Self {
            check: AccountCheck::new(pubkey),
        }
    }

    pub fn closed(mut self) -> Self {
        self.check.check_state = Some(AccountStateCheck::Closed);
        self
    }

    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.check.check_data = Some(data);
        self
    }

    pub fn executable(mut self, executable: bool) -> Self {
        self.check.check_executable = Some(executable);
        self
    }

    pub fn lamports(mut self, lamports: u64) -> Self {
        self.check.check_lamports = Some(lamports);
        self
    }

    pub fn owner(mut self, owner: &'a Pubkey) -> Self {
        self.check.check_owner = Some(owner);
        self
    }

    pub fn space(mut self, space: usize) -> Self {
        self.check.check_space = Some(space);
        self
    }

    pub fn data_slice(mut self, offset: usize, data: &'a [u8]) -> Self {
        self.check.check_data_slice = Some((offset, data));
        self
    }

    pub fn build(self) -> Check<'a> {
        Check::new(CheckType::ResultingAccount(self.check))
    }
}

struct CompareAccountFields {
    data: bool,
    executable: bool,
    lamports: bool,
    owner: bool,
    space: bool,
}

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
