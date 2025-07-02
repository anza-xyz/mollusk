//! Serde utilities for deserializing Solana accounts from JSON.

use {base64::Engine, serde::Deserialize, solana_account::Account, solana_pubkey::Pubkey};

/// Deserialize a Pubkey from a string.
pub fn pubkey_from_str<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<Pubkey>().map_err(serde::de::Error::custom)
}

/// Solana CLI/RPC JSON account.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAccount {
    pub lamports: u64,
    pub data: Vec<String>,
    #[serde(deserialize_with = "pubkey_from_str")]
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
    #[serde(default)]
    pub space: u64,
}

impl TryFrom<UiAccount> for Account {
    type Error = base64::DecodeError;

    fn try_from(ui_account: UiAccount) -> Result<Self, Self::Error> {
        let data = if ui_account.data.len() == 2 && ui_account.data[1] == "base64" {
            base64::engine::general_purpose::STANDARD.decode(&ui_account.data[0])?
        } else {
            Vec::new()
        };

        Ok(Account {
            lamports: ui_account.lamports,
            data,
            owner: ui_account.owner,
            executable: ui_account.executable,
            rent_epoch: ui_account.rent_epoch,
        })
    }
}

/// Solana CLI/RPC JSON account and pubkey.
#[derive(Debug, Deserialize)]
pub struct KeyedUiAccount {
    #[serde(deserialize_with = "pubkey_from_str")]
    pub pubkey: Pubkey,
    pub account: UiAccount,
}

impl TryFrom<KeyedUiAccount> for (Pubkey, Account) {
    type Error = base64::DecodeError;

    fn try_from(keyed: KeyedUiAccount) -> Result<Self, Self::Error> {
        Ok((keyed.pubkey, keyed.account.try_into()?))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, base64::Engine};

    #[test]
    fn test_deserialize_single_account() {
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

        let keyed_account: KeyedUiAccount = serde_json::from_str(&json_content).unwrap();
        let (loaded_pubkey, account) = keyed_account.try_into().unwrap();

        assert_eq!(loaded_pubkey, pubkey);
        assert_eq!(account.lamports, lamports);
        assert_eq!(account.data, data);
        assert_eq!(account.owner, owner);
        assert!(account.executable);
        assert_eq!(account.rent_epoch, rent_epoch);
    }

    #[test]
    fn test_deserialize_multiple_accounts() {
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

        let keyed_accounts: Vec<KeyedUiAccount> = serde_json::from_str(&json_content).unwrap();
        let accounts: Vec<(Pubkey, Account)> = keyed_accounts
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

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
}
