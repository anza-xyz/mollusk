use {solana_pubkey::Pubkey, std::collections::HashMap};

/// A simple map of vote accounts to their epoch stake.
///
/// Developers can work with this map directly to configure stake for testing.
/// The total epoch stake is calculated by summing all vote account stakes.
pub type EpochStake = HashMap<Pubkey, u64>;

/// Create an `EpochStake` instance with a few mocked-out vote accounts to
/// achieve the provided total stake.
pub fn create_mock_epoch_stake(target_total: u64) -> EpochStake {
    let mut epoch_stake = HashMap::new();

    if target_total == 0 {
        return epoch_stake;
    }

    let num_accounts = target_total.div_ceil(1_000_000_000);

    let base_stake = target_total / num_accounts;
    let remainder = target_total % base_stake;

    std::iter::repeat_n(base_stake, num_accounts as usize - 1)
        .chain(std::iter::once(base_stake + remainder))
        .for_each(|stake| {
            epoch_stake.insert(Pubkey::new_unique(), stake);
        });

    epoch_stake
}
