#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod address_lookup_table {
    solana_address::declare_id!("AddressLookupTab1e1111111111111111111111111");
}

pub mod bpf_loader {
    solana_address::declare_id!("BPFLoader2111111111111111111111111111111111");
}

pub mod bpf_loader_deprecated {
    solana_address::declare_id!("BPFLoader1111111111111111111111111111111111");
}

pub mod bpf_loader_upgradeable {
    solana_address::declare_id!("BPFLoaderUpgradeab1e11111111111111111111111");
}

pub mod compute_budget {
    solana_address::declare_id!("ComputeBudget111111111111111111111111111111");
}

pub mod config {
    solana_address::declare_id!("Config1111111111111111111111111111111111111");
}

pub mod ed25519_program {
    solana_address::declare_id!("Ed25519SigVerify111111111111111111111111111");
}

pub mod feature {
    solana_address::declare_id!("Feature111111111111111111111111111111111111");
}

/// A designated address for burning lamports.
///
/// Lamports credited to this address will be removed from the total supply
/// (burned) at the end of the current block.
pub mod incinerator {
    solana_address::declare_id!("1nc1nerator11111111111111111111111111111111");
}

pub mod loader_v4 {
    solana_address::declare_id!("LoaderV411111111111111111111111111111111111");
}

pub mod native_loader {
    solana_address::declare_id!("NativeLoader1111111111111111111111111111111");
}

pub mod secp256k1_program {
    solana_address::declare_id!("KeccakSecp256k11111111111111111111111111111");
}

pub mod secp256r1_program {
    solana_address::declare_id!("Secp256r1SigVerify1111111111111111111111111");
}

pub mod stake {
    pub mod config {
        solana_address::declare_deprecated_id!("StakeConfig11111111111111111111111111111111");
    }
    solana_address::declare_id!("Stake11111111111111111111111111111111111111");
}

pub mod system_program {
    solana_address::declare_id!("11111111111111111111111111111111");
}

pub mod vote {
    solana_address::declare_id!("Vote111111111111111111111111111111111111111");
}

pub mod sysvar {
    // Owner address for sysvar accounts
    solana_address::declare_id!("Sysvar1111111111111111111111111111111111111");
    pub mod clock {
        solana_address::declare_id!("SysvarC1ock11111111111111111111111111111111");
    }
    pub mod epoch_rewards {
        solana_address::declare_id!("SysvarEpochRewards1111111111111111111111111");
    }
    pub mod epoch_schedule {
        solana_address::declare_id!("SysvarEpochSchedu1e111111111111111111111111");
    }
    pub mod fees {
        solana_address::declare_id!("SysvarFees111111111111111111111111111111111");
    }
    pub mod instructions {
        solana_address::declare_id!("Sysvar1nstructions1111111111111111111111111");
    }
    pub mod last_restart_slot {
        solana_address::declare_id!("SysvarLastRestartS1ot1111111111111111111111");
    }
    pub mod recent_blockhashes {
        solana_address::declare_id!("SysvarRecentB1ockHashes11111111111111111111");
    }
    pub mod rent {
        solana_address::declare_id!("SysvarRent111111111111111111111111111111111");
    }
    pub mod rewards {
        solana_address::declare_id!("SysvarRewards111111111111111111111111111111");
    }
    pub mod slot_hashes {
        solana_address::declare_id!("SysvarS1otHashes111111111111111111111111111");
    }
    pub mod slot_history {
        solana_address::declare_id!("SysvarS1otHistory11111111111111111111111111");
    }
    pub mod stake_history {
        solana_address::declare_id!("SysvarStakeHistory1111111111111111111111111");
    }
}

pub mod zk_token_proof_program {
    solana_address::declare_id!("ZkTokenProof1111111111111111111111111111111");
}

pub mod zk_elgamal_proof_program {
    solana_address::declare_id!("ZkE1Gama1Proof11111111111111111111111111111");
}
