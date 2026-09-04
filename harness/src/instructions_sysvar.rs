use {
    solana_account::Account,
    solana_instruction::{BorrowedAccountMeta, BorrowedInstruction},
    solana_instructions_sysvar::construct_instructions_data,
    solana_message::SanitizedMessage,
    solana_pubkey::Pubkey,
};

pub fn keyed_account(message: &SanitizedMessage) -> (Pubkey, Account) {
    let account_keys = message.account_keys();
    let data = construct_instructions_data(
        message
            .program_instructions_iter()
            .map(|(program_id, instruction)| BorrowedInstruction {
                program_id,
                accounts: instruction
                    .accounts
                    .iter()
                    .map(|account_index| {
                        let account_index = usize::from(*account_index);
                        BorrowedAccountMeta {
                            pubkey: account_keys.get(account_index).unwrap(),
                            is_signer: message.is_signer(account_index),
                            is_writable: message.is_writable(account_index),
                        }
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
