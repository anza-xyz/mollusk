#![cfg(target_os = "solana")]

use {
    solana_account_info::AccountInfo,
    solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked},
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
};

solana_program_entrypoint::entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> Result<(), ProgramError> {
    let output_account = &accounts[0];

    // The last account is expected to be the instructions sysvar.
    let last_account_index = accounts.len() - 1;
    let ix_sysvar_account = &accounts[last_account_index];

    // Validate the instructions sysvar account.
    if !solana_instructions_sysvar::check_id(ix_sysvar_account.key) {
        Err(ProgramError::InvalidAccountOwner)?
    }

    // Validate the current instruction.

    let current_index = load_current_index_checked(ix_sysvar_account)?;
    let current_ix = load_instruction_at_checked(current_index as usize, ix_sysvar_account)?;

    if current_ix.program_id != *program_id {
        Err(ProgramError::InvalidInstructionData)?
    }

    for ((is_signer, is_writable), serialized_meta) in input
        .chunks(2)
        .map(|chunk| (chunk[0] != 0, chunk[1] != 0))
        .zip(current_ix.accounts.iter())
    {
        if is_signer != serialized_meta.is_signer || is_writable != serialized_meta.is_writable {
            Err(ProgramError::InvalidInstructionData)?
        }
    }

    if current_ix.data != input {
        Err(ProgramError::InvalidInstructionData)?
    }

    // // Write the entry for the current instruction.

    let entry_offset = current_index as usize * 35;

    // Write: program_id (32) | instruction_index (2) | executed (1)
    let mut data = output_account.try_borrow_mut_data()?;
    data[entry_offset..entry_offset + 32].copy_from_slice(program_id.as_ref());
    data[entry_offset + 32..entry_offset + 34].copy_from_slice(&current_index.to_le_bytes());
    data[entry_offset + 34] = 0; // executed = false

    // Mark previous entry as executed.
    if current_index > 0 {
        let prev_offset = (current_index - 1) as usize * 35;
        data[prev_offset + 34] = 1;
    }

    Ok(())
}
