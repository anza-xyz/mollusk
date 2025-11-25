use {
    solana_account::Account,
    solana_instruction::{BorrowedAccountMeta, BorrowedInstruction, Instruction},
    solana_instructions_sysvar::construct_instructions_data,
    solana_pubkey::Pubkey,
};

pub fn keyed_account(instructions: &[Instruction]) -> (Pubkey, Account) {
    let data = construct_instructions_data(
        instructions
            .iter()
            .map(|instruction| BorrowedInstruction {
                program_id: &instruction.program_id,
                accounts: instruction
                    .accounts
                    .iter()
                    .map(|meta| BorrowedAccountMeta {
                        pubkey: &meta.pubkey,
                        is_signer: meta.is_signer,
                        is_writable: meta.is_writable,
                    })
                    .collect(),
                data: &instruction.data,
            })
            .collect::<Vec<_>>()
            .as_slice(),
    );

    (
        solana_instructions_sysvar::ID,
        Account {
            lamports: 0,
            data,
            owner: solana_sysvar_id::ID,
            executable: false,
            rent_epoch: Default::default(),
        },
    )
}
