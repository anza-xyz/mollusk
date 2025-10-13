use {solana_pubkey::Pubkey, std::collections::HashMap};

/// A simple map of vote accounts to their epoch stake.
///
/// Developers can work with this map directly to configure stake for testing.
/// The total epoch stake is calculated by summing all vote account stakes.
pub type EpochStake = HashMap<Pubkey, u64>;

/// Create an `EpochStake` instance with a few mocked-out entries (vote accounts
/// with stake) to achieve the provided total stake.
pub fn create_mock_epoch_stake(target_total: u64) -> EpochStake {
    const BASE_STAKE_PER_ACCOUNT: u64 = 100_000_000_000; // 100 SOL

    let mut epoch_stake = HashMap::new();

    if target_total == 0 {
        return epoch_stake;
    }

    let num_accounts = target_total / BASE_STAKE_PER_ACCOUNT;
    let remainder = target_total % BASE_STAKE_PER_ACCOUNT;

    if num_accounts == 0 {
        epoch_stake.insert(Pubkey::new_unique(), target_total);
    } else {
        std::iter::repeat_n(BASE_STAKE_PER_ACCOUNT, num_accounts as usize - 1)
            .chain(std::iter::once(BASE_STAKE_PER_ACCOUNT + remainder))
            .for_each(|stake| {
                epoch_stake.insert(Pubkey::new_unique(), stake);
            });
    }

    epoch_stake
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_stake() {
        let epoch_stake = create_mock_epoch_stake(0);
        assert_eq!(epoch_stake.len(), 0);
        assert_eq!(epoch_stake.values().sum::<u64>(), 0);
    }

    #[test]
    fn test_num_accounts_zero() {
        // Target < 100 SOL, results in num_accounts = 0
        // Should create single account with full amount

        // 1 lamport
        let epoch_stake = create_mock_epoch_stake(1);
        assert_eq!(epoch_stake.len(), 1);
        assert_eq!(epoch_stake.values().sum::<u64>(), 1);

        // 50 SOL
        let epoch_stake = create_mock_epoch_stake(50_000_000_000);
        assert_eq!(epoch_stake.len(), 1);
        assert_eq!(epoch_stake.values().sum::<u64>(), 50_000_000_000);

        // 99.999999999 SOL
        let epoch_stake = create_mock_epoch_stake(99_999_999_999);
        assert_eq!(epoch_stake.len(), 1);
        assert_eq!(epoch_stake.values().sum::<u64>(), 99_999_999_999);
    }

    #[test]
    fn test_num_accounts_one() {
        // 100 SOL <= target < 200 SOL, results in num_accounts = 1

        // Exactly 100 SOL, no remainder
        let epoch_stake = create_mock_epoch_stake(100_000_000_000);
        assert_eq!(epoch_stake.len(), 1);
        assert_eq!(epoch_stake.values().sum::<u64>(), 100_000_000_000);
        assert!(epoch_stake.values().all(|&s| s == 100_000_000_000));

        // 150 SOL, with remainder
        let epoch_stake = create_mock_epoch_stake(150_000_000_000);
        assert_eq!(epoch_stake.len(), 1);
        assert_eq!(epoch_stake.values().sum::<u64>(), 150_000_000_000);
        assert!(epoch_stake.values().all(|&s| s == 150_000_000_000));

        // 199.999999999 SOL, with remainder
        let epoch_stake = create_mock_epoch_stake(199_999_999_999);
        assert_eq!(epoch_stake.len(), 1);
        assert_eq!(epoch_stake.values().sum::<u64>(), 199_999_999_999);
    }

    #[test]
    fn test_num_accounts_two() {
        // 200 SOL <= target < 300 SOL, results in num_accounts = 2

        // Exactly 200 SOL, no remainder -> [100, 100]
        let epoch_stake = create_mock_epoch_stake(200_000_000_000);
        assert_eq!(epoch_stake.len(), 2);
        assert_eq!(epoch_stake.values().sum::<u64>(), 200_000_000_000);
        assert!(epoch_stake.values().all(|&s| s == 100_000_000_000));

        // 250 SOL, with remainder -> [100, 150]
        let epoch_stake = create_mock_epoch_stake(250_000_000_000);
        assert_eq!(epoch_stake.len(), 2);
        assert_eq!(epoch_stake.values().sum::<u64>(), 250_000_000_000);
        let mut stakes: Vec<u64> = epoch_stake.values().copied().collect();
        stakes.sort();
        assert_eq!(stakes, vec![100_000_000_000, 150_000_000_000]);

        // 299.999999999 SOL, with remainder -> [100, 199.999999999]
        let epoch_stake = create_mock_epoch_stake(299_999_999_999);
        assert_eq!(epoch_stake.len(), 2);
        assert_eq!(epoch_stake.values().sum::<u64>(), 299_999_999_999);
        let mut stakes: Vec<u64> = epoch_stake.values().copied().collect();
        stakes.sort();
        assert_eq!(stakes, vec![100_000_000_000, 199_999_999_999]);
    }

    #[test]
    fn test_num_accounts_greater_than_two() {
        // target >= 300 SOL, results in num_accounts > 2

        // Exactly 300 SOL, no remainder -> [100, 100, 100]
        let epoch_stake = create_mock_epoch_stake(300_000_000_000);
        assert_eq!(epoch_stake.len(), 3);
        assert_eq!(epoch_stake.values().sum::<u64>(), 300_000_000_000);
        assert!(epoch_stake.values().all(|&s| s == 100_000_000_000));

        // 350 SOL, with remainder -> [100, 100, 150]
        let epoch_stake = create_mock_epoch_stake(350_000_000_000);
        assert_eq!(epoch_stake.len(), 3);
        assert_eq!(epoch_stake.values().sum::<u64>(), 350_000_000_000);
        let mut stakes: Vec<u64> = epoch_stake.values().copied().collect();
        stakes.sort();
        assert_eq!(
            stakes,
            vec![100_000_000_000, 100_000_000_000, 150_000_000_000]
        );

        // 1000 SOL, no remainder -> [100, 100, 100, 100, 100, 100, 100, 100, 100, 100]
        let epoch_stake = create_mock_epoch_stake(1_000_000_000_000);
        assert_eq!(epoch_stake.len(), 10);
        assert_eq!(epoch_stake.values().sum::<u64>(), 1_000_000_000_000);
        assert!(epoch_stake.values().all(|&s| s == 100_000_000_000));

        // 1234.567890123 SOL, with remainder
        let epoch_stake = create_mock_epoch_stake(1_234_567_890_123);
        assert_eq!(epoch_stake.len(), 12);
        assert_eq!(epoch_stake.values().sum::<u64>(), 1_234_567_890_123);
        let mut stakes: Vec<u64> = epoch_stake.values().copied().collect();
        stakes.sort();
        // Should have 11 accounts with 100 SOL and 1 account with 134.567890123 SOL
        assert_eq!(stakes.iter().filter(|&&s| s == 100_000_000_000).count(), 11);
        assert_eq!(stakes.iter().filter(|&&s| s == 134_567_890_123).count(), 1);
    }
}
