use solana_account::Account;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use spl_token::state::Mint;

pub struct MintAccountBuilder<'a> {
    mint_authority: Option<&'a Pubkey>,
    freeze_authority: Option<&'a Pubkey>,
    supply: u64,
    decimals: u8,
    is_initialized: bool,
}

impl<'a> MintAccountBuilder<'a> {
    pub fn new(supply: u64, decimals: u8) -> Self {
        Self {
            supply,
            decimals,
            mint_authority: None,
            freeze_authority: None,
            is_initialized: true,
        }
    }

    pub fn mint_authority(mut self, mint_authority: &'a Pubkey) -> Self {
        self.mint_authority = Some(mint_authority);
        self
    }

    pub fn freeze_authority(mut self, freeze_authority: &'a Pubkey) -> Self {
        self.freeze_authority = Some(freeze_authority);
        self
    }

    pub fn initialized(mut self, is_initialized: bool) -> Self {
        self.is_initialized = is_initialized;
        self
    }

    pub fn build(&self) -> Account {
        let data = {
            let mut data = vec![0; Mint::LEN];
            let state = Mint {
                mint_authority: self.mint_authority.cloned().into(),
                supply: self.supply,
                decimals: self.decimals,
                is_initialized: self.is_initialized,
                freeze_authority: self.freeze_authority.cloned().into(),
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
    fn test_mint_account_builder() {
        let mint_account = MintAccountBuilder::new(1_000_000_000, 9)
            .mint_authority(&Pubkey::new_unique())
            .freeze_authority(&Pubkey::new_unique())
            .build();

        assert_eq!(mint_account.owner, spl_token::id());
        assert!(mint_account.data.len() == Mint::LEN);
    }
}
