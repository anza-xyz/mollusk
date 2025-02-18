use {
    solana_account_info::{next_account_info, AccountInfo},
    solana_program_error::{ProgramError, ProgramResult},
    solana_pubkey::Pubkey,
};

solana_pubkey::declare_id!("MD24T7azhc2q9ZXaeskbLpmVA41k7StzTGgcfvGcpHj");

solana_program_entrypoint::entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    // Simply write the input data to the first account.
    let accounts_iter = &mut accounts.iter();

    let account_info = next_account_info(accounts_iter)?;

    if !account_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if account_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if input.len() > account_info.data_len() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    account_info.try_borrow_mut_data()?[..].copy_from_slice(input);

    Ok(())
}
