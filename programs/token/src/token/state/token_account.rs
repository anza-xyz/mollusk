use solana_account::Account;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use spl_token::state::{Account as TokenAccount, AccountState};

pub struct TokenAccountBuilder<'a> {
    mint: &'a Pubkey,
    owner: &'a Pubkey,
    amount: u64,
    delegate: Option<&'a Pubkey>,
    state: AccountState,
    is_native: Option<u64>,
    delegated_amount: u64,
    close_authority: Option<&'a Pubkey>,
}

impl<'a> TokenAccountBuilder<'a> {
    pub fn new(mint: &'a Pubkey, owner: &'a Pubkey) -> Self {
        Self {
            mint,
            owner,
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
        }
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn delegate(mut self, delegate: &'a Pubkey) -> Self {
        self.delegate = Some(delegate);
        self
    }

    pub fn state(mut self, state: AccountState) -> Self {
        self.state = state;
        self
    }

    pub fn native(mut self, native: u64) -> Self {
        self.is_native = Some(native);
        self
    }

    pub fn delegated_amount(mut self, delegated_amount: u64) -> Self {
        self.delegated_amount = delegated_amount;
        self
    }

    pub fn close_authority(mut self, close_authority: &'a Pubkey) -> Self {
        self.close_authority = Some(close_authority);
        self
    }

    pub fn build(&self) -> Account {
        let data = {
            let mut data = vec![0; TokenAccount::LEN];
            let state = TokenAccount {
                mint: *self.mint,
                owner: *self.owner,
                amount: self.amount,
                delegate: self.delegate.cloned().into(),
                state: self.state,
                is_native: self.is_native.into(),
                delegated_amount: self.delegated_amount,
                close_authority: self.close_authority.cloned().into(),
            };
            state.pack_into_slice(&mut data);
            data
        };

        let lamports = Rent::default().minimum_balance(data.len());

        Account {
            lamports,
            data,
            owner: spl_token::id(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_account_builder() {
        let mint_account = TokenAccountBuilder::new(&Pubkey::new_unique(), &Pubkey::new_unique())
            .amount(1000)
            .delegate(&Pubkey::new_unique())
            .build();

        assert_eq!(mint_account.owner, spl_token::id());
        assert!(mint_account.data.len() == TokenAccount::LEN);
    }
}
