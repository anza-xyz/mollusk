//! An account with an address: `(Pubkey, Account)`.

use {
    super::proto::{AcctState as ProtoAccount, SeedAddress as ProtoSeedAddress},
    solana_account::Account,
    solana_keccak_hasher::Hasher,
    solana_pubkey::Pubkey,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeedAddress {
    /// The seed address base (32 bytes).
    pub base: Vec<u8>,
    /// The seed path  (<= 32 bytes).
    pub seed: Vec<u8>,
    /// The seed address owner (32 bytes).
    pub owner: Vec<u8>,
}

impl From<ProtoSeedAddress> for SeedAddress {
    fn from(value: ProtoSeedAddress) -> Self {
        let ProtoSeedAddress { base, seed, owner } = value;
        Self { base, seed, owner }
    }
}

impl From<SeedAddress> for ProtoSeedAddress {
    fn from(value: SeedAddress) -> Self {
        let SeedAddress { base, seed, owner } = value;
        ProtoSeedAddress { base, seed, owner }
    }
}

impl From<ProtoAccount> for (Pubkey, Account, Option<SeedAddress>) {
    fn from(value: ProtoAccount) -> Self {
        let ProtoAccount {
            address,
            owner,
            lamports,
            data,
            executable,
            rent_epoch,
            seed_addr,
        } = value;

        let pubkey_bytes: [u8; 32] = address.try_into().expect("Invalid bytes for pubkey");
        let pubkey = Pubkey::new_from_array(pubkey_bytes);

        let owner_bytes: [u8; 32] = owner.try_into().expect("Invalid bytes for owner");
        let owner = Pubkey::new_from_array(owner_bytes);

        (
            pubkey,
            Account {
                data,
                executable,
                lamports,
                owner,
                rent_epoch,
            },
            seed_addr.map(Into::into),
        )
    }
}

impl From<(Pubkey, Account, Option<SeedAddress>)> for ProtoAccount {
    fn from(value: (Pubkey, Account, Option<SeedAddress>)) -> Self {
        let Account {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        } = value.1;

        ProtoAccount {
            address: value.0.to_bytes().to_vec(),
            owner: owner.to_bytes().to_vec(),
            lamports,
            data,
            executable,
            rent_epoch,
            seed_addr: value.2.map(Into::into),
        }
    }
}

impl From<(Pubkey, Account)> for ProtoAccount {
    fn from(value: (Pubkey, Account)) -> Self {
        let Account {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        } = value.1;

        ProtoAccount {
            address: value.0.to_bytes().to_vec(),
            owner: owner.to_bytes().to_vec(),
            lamports,
            data,
            executable,
            rent_epoch,
            seed_addr: None,
        }
    }
}

pub(crate) fn hash_proto_accounts(hasher: &mut Hasher, accounts: &[ProtoAccount]) {
    for account in accounts {
        hasher.hash(&account.address);
        hasher.hash(&account.owner);
        hasher.hash(&account.lamports.to_le_bytes());
        hasher.hash(&account.data);
        hasher.hash(&[account.executable as u8]);
        hasher.hash(&account.rent_epoch.to_le_bytes());
        if let Some(seed_addr) = &account.seed_addr {
            hasher.hash(&seed_addr.base);
            hasher.hash(&seed_addr.seed);
            hasher.hash(&seed_addr.owner);
        }
    }
}
