# Mollusk Account Fetchers

Simple, lightweight account fetching utilities for Mollusk.

## Overview

Two standalone account fetchers that can be used with or without Mollusk:

* **`mollusk-svm-account-fetcher-fs`** - Load accounts from local JSON files
* **`mollusk-svm-account-fetcher-rpc`** - Fetch accounts via Solana JSON-RPC

Both fetchers are designed to be minimal and composable, returning standard
`Vec<(Pubkey, Account)>` that can be used directly with Mollusk's instruction
processing methods.

## File System Fetcher

The file system fetcher loads accounts from JSON files that match the format
produced by `solana account -o json`.

You can dump accounts to JSON files using the Solana JSON-RPC:

```bash
solana account <ADDRESS> --output json --output-file account.json
```

```rust
use mollusk_svm_account_fetcher_fs::{
  load_account_from_json_file,
  load_multiple_accounts_from_directory,
  load_multiple_accounts_from_json_file,
};

// Load a single account from a file.
let (pubkey, account) = load_account_from_json_file("./account.json")?;

// Load multiple accounts from an array in a file.
let accounts = load_multiple_accounts_from_json_file("./accounts.json")?;

// Recursively load all JSON files from a directory.
let accounts = load_multiple_accounts_from_directory("./fixtures")?;
```

Each account JSON should match the exact format produced by the Solana CLI:

```json
{
  "pubkey": "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
  "account": {
    "lamports": 1000000000,
    "data": ["SGVsbG8gV29ybGQh", "base64"],
    "owner": "11111111111111111111111111111111",
    "executable": false,
    "rentEpoch": 0,
    "space": 13
  }
}
```

An array of accounts can be stored in a single file:

```json
[
  {
    "pubkey": "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "account": {
      "lamports": 1000000000,
      "data": ["SGVsbG8gV29ybGQh", "base64"],
      "owner": "11111111111111111111111111111111",
      "executable": false,
      "rentEpoch": 0,
      "space": 13
    }
  },
  {
    "pubkey": "EkBn7qWtupWhZvCXYAKcvkLEKqMWPHdcV84QdE9MLLan",
    "account": {
      "lamports": 2000000000,
      "data": ["AQIDBAUGBwg=", "base64"],
      "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      "executable": false,
      "rentEpoch": 0,
      "space": 8
    }
  }
]
```

## RPC Fetcher

The RPC fetcher provides a minimal client for fetching accounts from Solana
JSON-RPC endpoints without the overhead of `solana-client`. It reimplements
only the `getAccountInfo` and `getMultipleAccounts` methods using `reqwest`.

The RPC fetcher imposes the following default RPC behavior:
* Always requests accounts with `base64` encoding
* Automatically decodes the base64 data into `Vec<u8>` for the `Account` struct
* Returns `None` for accounts that don't exist on-chain

```rust
use mollusk_svm_account_fetcher_rpc::{
    fetch_account, 
    fetch_accounts,
    fetch_accounts_with_default
};

// Fetch a single account (returns `Option<Account>`).
let account = fetch_account(
    "https://api.mainnet-beta.solana.com", 
    &pubkey
).await?;

// Fetch multiple accounts (returns `Vec<(Pubkey, Account)>`).
// Always returns one account per pubkey, using `Account::default()` for
// non-existent ones.
let accounts = fetch_accounts(
    "https://api.mainnet-beta.solana.com",
    &[pubkey1, pubkey2, pubkey3]
).await?;

// Fetch with custom defaults for missing accounts.
// Allows you to specify how default accounts should be created.
let accounts = fetch_accounts_with_default(
    "https://api.mainnet-beta.solana.com",
    &[pubkey1, pubkey2, pubkey3],
    |_pubkey| Account::default()  // <-- Your custom default logic
).await?;
```

These functions create a new `RpcClient` internally for each call. If you're
making multiple requests, consider using the `RpcClient` directly.

```rust
let client = RpcClient::new("https://api.mainnet-beta.solana.com");
```
