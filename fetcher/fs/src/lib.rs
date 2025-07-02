//! File system account fetcher.
//!
//! This crate provides utilities to load Solana accounts from JSON files
//! that match the format produced by `solana account -o json`.

use {
    mollusk_svm_account_fetcher_serde::KeyedUiAccount,
    solana_account::Account,
    solana_pubkey::Pubkey,
    std::{fs, path::Path},
    thiserror::Error,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Load a single account from a JSON file.
///
/// The file should contain a single account object in the same format
/// as produced by `solana account -o json`.
pub fn load_account_from_json_file<P: AsRef<Path>>(path: P) -> Result<(Pubkey, Account), Error> {
    let content = fs::read_to_string(path)?;
    let keyed_account: KeyedUiAccount = serde_json::from_str(&content)?;
    Ok(keyed_account.try_into()?)
}

/// Load multiple accounts from a JSON file containing an array.
///
/// The file should contain an array of account objects, each in the same format
/// as produced by `solana account -o json`.
pub fn load_multiple_accounts_from_json_file<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<(Pubkey, Account)>, Error> {
    let content = fs::read_to_string(path)?;
    let keyed_accounts: Vec<KeyedUiAccount> = serde_json::from_str(&content)?;
    keyed_accounts
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Load accounts from multiple files in a directory.
///
/// This function will recursively search for `.json` files in the given
/// directory and attempt to load accounts from each file. Non-JSON files are
/// skipped. If a JSON file fails to parse, the operation will fail with an
/// error.
pub fn load_multiple_accounts_from_directory<P: AsRef<Path>>(
    dir: P,
) -> Result<Vec<(Pubkey, Account)>, Error> {
    let mut all_accounts = Vec::new();
    let dir = dir.as_ref();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively load from any subdirectories.
            let sub_accounts = load_multiple_accounts_from_directory(&path)?;
            all_accounts.extend(sub_accounts);
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            // Try to load as an array first, then try as a single account.
            match load_multiple_accounts_from_json_file(&path) {
                Ok(accounts) => all_accounts.extend(accounts),
                Err(_) => {
                    let account = load_account_from_json_file(&path)?;
                    all_accounts.push(account);
                }
            }
        }
    }

    Ok(all_accounts)
}

#[cfg(test)]
mod tests {
    use {super::*, base64::Engine, std::io::Write, tempfile::TempDir};

    #[test]
    fn test_load_single_account() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("account.json");

        let pubkey = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let lamports = 1234567890;
        let rent_epoch = 42;
        let data = vec![1, 2, 3, 4, 5];
        let data_base64 = base64::engine::general_purpose::STANDARD.encode(&data);
        let space = data.len();

        let json_content = format!(
            r#"{{
            "pubkey": "{pubkey}",
            "account": {{
                "lamports": {lamports},
                "data": ["{data_base64}", "base64"],
                "owner": "{owner}",
                "executable": true,
                "rentEpoch": {rent_epoch},
                "space": {space}
            }}
        }}"#
        );

        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(json_content.as_bytes()).unwrap();

        let (loaded_pubkey, account) = load_account_from_json_file(&file_path).unwrap();
        assert_eq!(loaded_pubkey, pubkey);
        assert_eq!(account.lamports, lamports);
        assert_eq!(account.data, data);
        assert_eq!(account.owner, owner);
        assert!(account.executable);
        assert_eq!(account.rent_epoch, rent_epoch);
    }

    #[test]
    fn test_load_multiple_accounts() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("accounts.json");

        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        let owner1 = Pubkey::new_unique();
        let owner2 = Pubkey::new_unique();
        let lamports1 = 1000000;
        let lamports2 = 2000000;
        let rent_epoch1 = 100;
        let rent_epoch2 = 200;
        let data1: Vec<u8> = vec![];
        let data2 = vec![1, 2, 3];
        let data2_base64 = base64::engine::general_purpose::STANDARD.encode(&data2);
        let space1 = data1.len();
        let space2 = data2.len();

        let json_content = format!(
            r#"[
            {{
                "pubkey": "{pubkey1}",
                "account": {{
                    "lamports": {lamports1},
                    "data": ["", "base64"],
                    "owner": "{owner1}",
                    "executable": false,
                    "rentEpoch": {rent_epoch1},
                    "space": {space1}
                }}
            }},
            {{
                "pubkey": "{pubkey2}",
                "account": {{
                    "lamports": {lamports2},
                    "data": ["{data2_base64}", "base64"],
                    "owner": "{owner2}",
                    "executable": true,
                    "rentEpoch": {rent_epoch2},
                    "space": {space2}
                }}
            }}
        ]"#
        );

        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(json_content.as_bytes()).unwrap();

        let accounts = load_multiple_accounts_from_json_file(&file_path).unwrap();
        assert_eq!(accounts.len(), 2);

        assert_eq!(accounts[0].0, pubkey1);
        assert_eq!(accounts[0].1.lamports, lamports1);
        assert_eq!(accounts[0].1.data, data1);
        assert_eq!(accounts[0].1.owner, owner1);
        assert!(!accounts[0].1.executable);
        assert_eq!(accounts[0].1.rent_epoch, rent_epoch1);

        assert_eq!(accounts[1].0, pubkey2);
        assert_eq!(accounts[1].1.lamports, lamports2);
        assert_eq!(accounts[1].1.data, data2);
        assert_eq!(accounts[1].1.owner, owner2);
        assert!(accounts[1].1.executable);
        assert_eq!(accounts[1].1.rent_epoch, rent_epoch2);
    }

    #[test]
    fn test_load_directory_with_invalid_json() {
        let dir = TempDir::new().unwrap();

        // Create a file with .json extension but invalid contents.
        let mut file = fs::File::create(dir.path().join("invalid.json")).unwrap();
        file.write_all(b"{ invalid json }").unwrap();

        let result = load_multiple_accounts_from_directory(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_directory_skip_non_json() {
        let dir = TempDir::new().unwrap();

        // Create a valid JSON account file.
        let pubkey = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let lamports = 987654321;
        let rent_epoch = 555;
        let data = vec![10, 20, 30];
        let data_base64 = base64::engine::general_purpose::STANDARD.encode(&data);

        let valid_json = format!(
            r#"{{
            "pubkey": "{pubkey}",
            "account": {{
                "lamports": {lamports},
                "data": ["{data_base64}", "base64"],
                "owner": "{owner}",
                "executable": false,
                "rentEpoch": {rent_epoch},
                "space": {}
            }}
        }}"#,
            data.len()
        );
        let mut file = fs::File::create(dir.path().join("account.json")).unwrap();
        file.write_all(valid_json.as_bytes()).unwrap();

        // Create non-JSON files (should be skipped).
        fs::File::create(dir.path().join("readme.txt")).unwrap();
        fs::File::create(dir.path().join("config.toml")).unwrap();
        fs::File::create(dir.path().join("data.bin")).unwrap();

        // Should only load the one JSON file.
        let accounts = load_multiple_accounts_from_directory(dir.path()).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].0, pubkey);
        assert_eq!(accounts[0].1.lamports, lamports);
        assert_eq!(accounts[0].1.data, data);
        assert_eq!(accounts[0].1.owner, owner);
        assert!(!accounts[0].1.executable);
        assert_eq!(accounts[0].1.rent_epoch, rent_epoch);
    }
}
