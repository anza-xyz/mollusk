#[cfg(feature = "syscalls")]
use crate::Instruction;
#[cfg(target_os = "solana")]
pub use {
    crate::{AccountMeta, ProcessedSiblingInstruction},
    solana_define_syscall::definitions::sol_get_stack_height,
    solana_pubkey::Pubkey,
};

#[cfg(target_os = "solana")]
#[deprecated(
    since = "3.1.0",
    note = "Use `solana_define_syscall::definitions::get_processed_sibling_instruction` instead"
)]
pub unsafe fn sol_get_processed_sibling_instruction(
    index: u64,
    meta: *mut ProcessedSiblingInstruction,
    program_id: *mut Pubkey,
    data: *mut u8,
    accounts: *mut AccountMeta,
) -> u64 {
    unsafe {
        solana_define_syscall::definitions::sol_get_processed_sibling_instruction(
            index,
            meta as *mut u8,
            program_id as *mut u8,
            data,
            accounts as *mut u8,
        )
    }
}

/// Returns a sibling instruction from the processed sibling instruction list.
///
/// The processed sibling instruction list is a reverse-ordered list of
/// successfully processed sibling instructions. For example, given the call flow:
///
/// A
/// B -> C -> D
/// B -> E
/// B -> F
///
/// Then B's processed sibling instruction list is: `[A]`
/// Then F's processed sibling instruction list is: `[E, C]`
#[cfg(feature = "syscalls")]
pub fn get_processed_sibling_instruction(index: usize) -> Option<Instruction> {
    #[cfg(target_os = "solana")]
    {
        let mut meta = ProcessedSiblingInstruction::default();
        let mut program_id = solana_pubkey::Pubkey::default();
        let mut account_meta = AccountMeta::default();

        if 1 == unsafe {
            solana_define_syscall::definitions::sol_get_processed_sibling_instruction(
                index as u64,
                &mut meta as *mut _ as *mut u8,
                &mut program_id as *mut _ as *mut u8,
                &mut u8::default(),
                &mut account_meta as *mut _ as *mut u8,
            )
        } {
            let mut data = std::vec::Vec::new();
            let mut accounts = std::vec::Vec::new();
            data.resize_with(meta.data_len as usize, u8::default);
            accounts.resize_with(meta.accounts_len as usize, AccountMeta::default);

            let _ = unsafe {
                solana_define_syscall::definitions::sol_get_processed_sibling_instruction(
                    index as u64,
                    &mut meta as *mut _ as *mut u8,
                    &mut program_id as *mut _ as *mut u8,
                    data.as_mut_ptr(),
                    accounts.as_mut_ptr() as *mut u8,
                )
            };

            Some(Instruction::new_with_bytes(program_id, &data, accounts))
        } else {
            None
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(index);
        // Same value used in `solana_sysvar::program_stubs`.
        None
    }
}

/// Get the current stack height.
///
/// Transaction-level instructions are height
/// [`crate::TRANSACTION_LEVEL_STACK_HEIGHT`]`, fist invoked inner instruction
/// is height `TRANSACTION_LEVEL_STACK_HEIGHT + 1`, and so forth.
#[cfg(feature = "syscalls")]
pub fn get_stack_height() -> usize {
    #[cfg(target_os = "solana")]
    unsafe {
        sol_get_stack_height() as usize
    }

    #[cfg(not(target_os = "solana"))]
    // Same value used in `solana_sysvar::program_stubs`.
    0
}
