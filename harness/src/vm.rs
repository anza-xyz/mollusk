//! Virtual Machine API for using Mollusk with custom VMs.

/// A virtual machine compatible with the Solana calling convention.
pub trait SolanaVM {}

pub mod agave {
    use super::SolanaVM;

    /// The SBPF virtual machine used in Anza's Agave validator.
    pub struct AgaveVM {}

    impl SolanaVM for AgaveVM {}
}
