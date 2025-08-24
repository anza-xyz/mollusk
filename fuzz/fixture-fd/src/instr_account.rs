//! Instruction account.

use {
    super::proto::InstrAcct as ProtoInstrAccount, solana_keccak_hasher::Hasher,
    solana_transaction_context::InstructionAccount,
};

impl From<ProtoInstrAccount> for InstructionAccount {
    fn from(value: ProtoInstrAccount) -> Self {
        let ProtoInstrAccount {
            index,
            is_writable,
            is_signer,
        } = value;
        InstructionAccount::new(index as u16, is_signer, is_writable)
    }
}

impl From<InstructionAccount> for ProtoInstrAccount {
    fn from(value: InstructionAccount) -> Self {
        Self {
            index: value.index_in_transaction as u32,
            is_signer: value.is_signer(),
            is_writable: value.is_writable(),
        }
    }
}

pub(crate) fn hash_proto_instr_accounts(hasher: &mut Hasher, instr_accounts: &[ProtoInstrAccount]) {
    for account in instr_accounts {
        hasher.hash(&account.index.to_le_bytes());
        hasher.hash(&[account.is_signer as u8]);
        hasher.hash(&[account.is_writable as u8]);
    }
}
