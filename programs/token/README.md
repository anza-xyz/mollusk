# Mollusk SVM Programs Token

## Examples

Init after close account

```rust
use {
    mollusk_svm::{result::Check, Mollusk},
    mollusk_svm_programs_token::token::state::{mint::MintAccountBuilder, token_account::TokenAccountBuilder},
    solana_sdk::{
        account::{Account, ReadableAccount},
        pubkey::Pubkey,
        system_instruction, system_program,
    }
};

#[test]
fn success_init_after_close_account() {
    let mollusk = Mollusk::new(&spl_token::id(), "spl_token");

    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let account = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 9;

    let owner_account = Account::new(1_000_000_000, 0, &system_program::id());
    let mint_account =  MintAccountBuilder::new(0, decimals).build();
    let token_account = TokenAccountBuilder::new(&mint, &owner)
        .build();

    let expected_destination_lamports = token_account.lamports();

    mollusk.process_and_validate_instruction_chain(
        &[
            (
                &spl_token::instruction::close_account(&spl_token::id(), &account, &destination, &owner, &[])
                    .unwrap(),
                &[Check::success()],
            ),
            (
                &system_instruction::create_account(
                    &owner,
                    &account,
                    1_000_000_000,
                    Account::LEN as u64,
                    &spl_token::id(),
                ),
                &[Check::success()],
            ),
            (
                &spl_token::instruction::initialize_account(&spl_token::id(), &account, &mint, &owner)
                    .unwrap(),
                &[
                    Check::success(),
                    // Account successfully re-initialized.
                    Check::account(&account)
                        .data(setup::setup_token_account(&mint, &owner, 0).data())
                        .owner(&spl_token::id())
                        .build(),
                    // The destination should have the lamports from the closed account.
                    Check::account(&destination)
                        .lamports(expected_destination_lamports)
                        .build(),
                ],
            ),
        ],
        &[
            (mint, mint_account),
            (account, token_account),
            (owner, owner_account),
            (destination, Account::default()),
            mollusk.sysvars.keyed_account_for_rent_sysvar(),
        ],
    );
}
```