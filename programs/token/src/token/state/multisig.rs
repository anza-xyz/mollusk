use solana_account::Account;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use spl_token::{instruction::MAX_SIGNERS, state::Multisig};

pub struct MultisigAccountBuilder {
    m: u8,
    n: u8,
    is_initialized: bool,
    signers: [Pubkey; MAX_SIGNERS],
}

impl MultisigAccountBuilder {
    pub fn new(m: u8, n: u8) -> Self {
        Self {
            m,
            n,
            is_initialized: true,
            signers: [Pubkey::new_from_array([0u8; 32]); MAX_SIGNERS],
        }
    }

    pub fn initialized(mut self, is_initialized: bool) -> Self {
        self.is_initialized = is_initialized;
        self
    }

    pub fn signers(mut self, signers: [Pubkey; MAX_SIGNERS]) -> Self {
        self.signers = signers;
        self
    }

    pub fn build(&self) -> Account {
        let data = {
            let mut data = vec![0; Multisig::LEN];
            let state = Multisig {
                m: self.m,
                n: self.n,
                is_initialized: self.is_initialized,
                signers: self.signers,
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
    fn test_multisig_account_builder() {
        let mint_account = MultisigAccountBuilder::new(2, 3).build();

        assert_eq!(mint_account.owner, spl_token::id());
        assert!(mint_account.data.len() == Multisig::LEN);
    }
}
