//! The rent check the runtime applies after execution.

use {
    mollusk_svm_result::config::Config,
    solana_account::Account,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_svm::rent_calculator::{
        check_rent_state_with_account, get_post_exec_account_rent_state,
        get_pre_exec_account_rent_state, RentState,
    },
    solana_transaction_context::IndexOfAccount,
    solana_transaction_error::TransactionResult,
};

pub(crate) fn check_transitions(
    rent: &Rent,
    config: &Config,
    relax_post_exec_min_balance_check: bool,
    pre: &[(Pubkey, Account)],
    post: &[(Pubkey, Account)],
) -> TransactionResult<()> {
    if !config.rent_exempt_checks {
        return Ok(());
    }

    for (index, (pubkey, post_account)) in post.iter().enumerate() {
        let Some((_, pre_account)) = pre.iter().find(|(key, _)| key == pubkey) else {
            continue;
        };

        let pre_state = get_pre_exec_account_rent_state(
            pre_account.lamports,
            pre_account.data.len(),
            rent.minimum_balance(pre_account.data.len()),
            relax_post_exec_min_balance_check,
        );

        let relax_rent_exempt_criteria = relax_post_exec_min_balance_check
            && pre_account.data.len() >= post_account.data.len()
            && pre_state == RentState::RentExempt
            && pre_account.owner == post_account.owner;

        let post_state = get_post_exec_account_rent_state(
            post_account.lamports,
            post_account.data.len(),
            rent.minimum_balance(post_account.data.len()),
            &pre_state,
            pre_account.lamports,
            relax_rent_exempt_criteria,
        );

        let Err(err) =
            check_rent_state_with_account(&pre_state, &post_state, pubkey, index as IndexOfAccount)
        else {
            continue;
        };

        let msg = format!(
            "CHECK FAILED: rent\n  Account {} was left in a rent state the runtime rejects \
             (lamports: {}, data_len: {}): {}.\n  Set `config.rent_exempt_checks = false` to \
             disable this check.",
            pubkey,
            post_account.lamports,
            post_account.data.len(),
            err,
        );
        if config.panic {
            panic!("{}", msg);
        } else if config.verbose {
            eprintln!("{}", msg);
        }
        return Err(err);
    }
    Ok(())
}
