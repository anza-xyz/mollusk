#![cfg(target_os = "solana")]

use {solana_account_info::AccountInfo, solana_program_error::ProgramError, solana_pubkey::Pubkey};

extern "C" {
    fn sol_get_epoch_stake(vote_address: *const u8) -> u64;
}

unsafe fn get_epoch_total_stake() -> u64 {
    sol_get_epoch_stake(std::ptr::null::<Pubkey>() as *const u8)
}

unsafe fn get_epoch_stake_for_vote_account(vote_address: &Pubkey) -> u64 {
    sol_get_epoch_stake(vote_address as *const _ as *const u8)
}

solana_program_entrypoint::entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> Result<(), ProgramError> {
    let vote_address = Pubkey::new_from_array(input.try_into().unwrap());

    let total_stake = unsafe { get_epoch_total_stake() };
    let vote_account_stake = unsafe { get_epoch_stake_for_vote_account(&vote_address) };

    let mut data = accounts[0].try_borrow_mut_data()?;
    data[0..8].copy_from_slice(&total_stake.to_le_bytes());
    data[8..16].copy_from_slice(&vote_account_stake.to_le_bytes());

    Ok(())
}
