//! An account with an address: `(Pubkey, AccountSharedData)`.

use {
    super::proto::AcctState as ProtoAccount,
    solana_sdk::{
        account::{Account, AccountSharedData},
        pubkey::Pubkey,
    },
};

impl From<ProtoAccount> for (Pubkey, AccountSharedData) {
    fn from(value: ProtoAccount) -> Self {
        let ProtoAccount {
            address,
            owner,
            lamports,
            data,
            executable,
            rent_epoch,
        } = value;

        let pubkey_bytes: [u8; 32] = address.try_into().expect("Invalid bytes for pubkey");
        let pubkey = Pubkey::new_from_array(pubkey_bytes);

        let owner_bytes: [u8; 32] = owner.try_into().expect("Invalid bytes for owner");
        let owner = Pubkey::new_from_array(owner_bytes);

        (
            pubkey,
            AccountSharedData::from(Account {
                data,
                executable,
                lamports,
                owner,
                rent_epoch,
            }),
        )
    }
}

impl From<(Pubkey, AccountSharedData)> for ProtoAccount {
    fn from(value: (Pubkey, AccountSharedData)) -> Self {
        let Account {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        } = value.1.into();

        ProtoAccount {
            address: value.0.to_bytes().to_vec(),
            owner: owner.to_bytes().to_vec(),
            lamports,
            data,
            executable,
            rent_epoch,
        }
    }
}
