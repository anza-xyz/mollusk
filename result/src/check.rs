//! Check system for validating individual instruction results.

use {
    crate::{
        config::{CheckContext, Config, compare, throw},
        types::{InstructionResult, ProgramResult},
    },
    solana_account::{Account, ReadableAccount},
    solana_instruction::error::InstructionError,
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
};

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
    /// Check that all accounts are rent exempt.
    AllRentExempt,
    /// Custom check for accounts.
    Custom((Box<dyn Fn(&[(Pubkey, Account)]) -> bool>, &'static str)),
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

    /// Check that all resulting accounts are rent exempt.
    pub fn all_rent_exempt() -> Self {
        Check::new(CheckType::AllRentExempt)
    }

    /// Custom check for accounts.
    pub fn custom<F>(function: F, function_name: &'static str) -> Self
    where
        F: Fn(&[(Pubkey, Account)]) -> bool + 'static,
    {
        Check::new(CheckType::Custom((Box::new(function), function_name)))
    }
}

enum AccountStateCheck {
    Closed,
    RentExempt,
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

    pub fn rent_exempt(mut self) -> Self {
        self.check.check_state = Some(AccountStateCheck::RentExempt);
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

impl InstructionResult {
    /// Perform checks on the instruction result with a custom context.
    /// See `CheckContext` for more details.
    ///
    /// Note: `Mollusk` implements `CheckContext`, in case you don't want to
    /// define a custom context.
    pub fn run_checks<C: CheckContext>(
        &self,
        checks: &[Check],
        config: &Config,
        context: &C,
    ) -> bool {
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
                                    resulting_account == &Default::default(),
                                );
                            }
                            AccountStateCheck::RentExempt => {
                                pass &= compare!(
                                    c,
                                    "account_rent_exempt",
                                    true,
                                    context.is_rent_exempt(
                                        resulting_account.lamports,
                                        resulting_account.data.len(),
                                        resulting_account.owner,
                                    ),
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
                CheckType::AllRentExempt => {
                    for (pubkey, account) in &self.resulting_accounts {
                        let is_rent_exempt = context.is_rent_exempt(
                            account.lamports(),
                            account.data().len(),
                            account.owner,
                        );
                        if !is_rent_exempt {
                            pass &= throw!(
                                c,
                                "Account {} is not rent exempt after execution (lamports: {}, \
                                 data_len: {})",
                                pubkey,
                                account.lamports(),
                                account.data().len()
                            );
                        }
                    }
                }
                CheckType::Custom((function, function_name))=>{
                        println!("Calling {}", *function_name);
                        pass &= function(&self.resulting_accounts);
                }
            }
        }
        pass
    }
}