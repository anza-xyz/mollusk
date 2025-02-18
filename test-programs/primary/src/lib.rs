use {
    solana_account_info::{next_account_info, AccountInfo},
    solana_cpi::invoke,
    solana_instruction::{AccountMeta, Instruction},
    solana_program_error::{ProgramError, ProgramResult},
    solana_pubkey::{Pubkey, PUBKEY_BYTES},
};

solana_pubkey::declare_id!("239vxAL9Q7e3uLoinJpJ873r3bvT9sPFxH7yekwPppNF");

solana_program_entrypoint::entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    match input.split_first() {
        Some((0, _)) => {
            // No-op.
        }
        Some((1, rest)) => {
            // Simply write the remaining data to the first account.
            let account_info = next_account_info(accounts_iter)?;

            if !account_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            if rest.len() > account_info.data_len() {
                return Err(ProgramError::AccountDataTooSmall);
            }

            account_info.try_borrow_mut_data()?[..].copy_from_slice(rest);
        }
        Some((2, rest)) if rest.len() == 8 => {
            // Transfer from the first account to the second.
            let payer_info = next_account_info(accounts_iter)?;
            let recipient_info = next_account_info(accounts_iter)?;
            let _system_program = next_account_info(accounts_iter)?;

            if !payer_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            let lamports = u64::from_le_bytes(rest.try_into().unwrap());

            invoke(
                &solana_system_interface::instruction::transfer(
                    payer_info.key,
                    recipient_info.key,
                    lamports,
                ),
                &[payer_info.clone(), recipient_info.clone()],
            )?;
        }
        Some((3, _)) => {
            // Close the first account and burn its lamports.
            let account_info = next_account_info(accounts_iter)?;
            let incinerator_info = next_account_info(accounts_iter)?;
            let _system_program = next_account_info(accounts_iter)?;

            if !account_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            account_info.realloc(0, true)?;
            account_info.assign(&solana_sdk_ids::system_program::id());

            let lamports = account_info.lamports();

            invoke(
                &solana_system_interface::instruction::transfer(
                    account_info.key,
                    &solana_sdk_ids::incinerator::id(),
                    lamports,
                ),
                &[account_info.clone(), incinerator_info.clone()],
            )?;
        }
        Some((4, rest)) if rest.len() >= PUBKEY_BYTES => {
            // Invoke the "CPI Target" test program, which will write the rest
            // of the input data to the first account (after the provided
            // program ID).
            let account_info = next_account_info(accounts_iter)?;

            let (program_id_bytes, data) = rest.split_at(PUBKEY_BYTES);

            let program_id = Pubkey::new_from_array(program_id_bytes.try_into().unwrap());
            let instruction = Instruction::new_with_bytes(
                program_id,
                data,
                vec![AccountMeta::new(*account_info.key, true)],
            );

            invoke(&instruction, &[account_info.clone()])?;
        }
        Some((5, _)) => {
            // Load the same account twice and assert both infos share the
            // same privilege level.
            let first_info = next_account_info(accounts_iter)?;
            let second_info = next_account_info(accounts_iter)?;

            if first_info.key != second_info.key {
                return Err(ProgramError::InvalidArgument);
            }

            if first_info.is_writable != second_info.is_writable {
                return Err(ProgramError::Immutable);
            }

            if first_info.is_signer != second_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}
