//! Minimal JSON-RPC account fetcher.
//!
//! This crate provides a lightweight RPC client for fetching Solana accounts
//! without the overhead of the full solana-client library.

use {
    mollusk_svm_account_fetcher_serde::UiAccount,
    serde::{Deserialize, Serialize},
    solana_account::Account,
    solana_pubkey::Pubkey,
    thiserror::Error,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("RPC error: {code}: {message}")]
    Rpc { code: i64, message: String },
}

/// Minimal RPC client for fetching Solana accounts.
pub struct RpcClient {
    url: String,
    client: reqwest::Client,
}

impl RpcClient {
    /// Create a new RPC client with the given endpoint URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Fetch a single account.
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>, Error> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "getAccountInfo",
            params: serde_json::json!([
                pubkey.to_string(),
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]),
        };

        let response: RpcResponse<RpcAccountInfo> = self.send_request(request).await?;

        match response.result.value {
            Some(ui_account) => Ok(Some(ui_account.try_into()?)),
            None => Ok(None),
        }
    }

    /// Fetch multiple accounts.
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, Error> {
        let pubkey_strings: Vec<String> = pubkeys.iter().map(|p| p.to_string()).collect();

        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "getMultipleAccounts",
            params: serde_json::json!([
                pubkey_strings,
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]),
        };

        let response: RpcResponse<RpcMultipleAccounts> = self.send_request(request).await?;

        response
            .result
            .value
            .into_iter()
            .map(|opt_account| match opt_account {
                Some(ui_account) => Ok(Some(ui_account.try_into()?)),
                None => Ok(None),
            })
            .collect()
    }

    async fn send_request<T: for<'de> Deserialize<'de>>(
        &self,
        request: RpcRequest,
    ) -> Result<RpcResponse<T>, Error> {
        let response = self.client.post(&self.url).json(&request).send().await?;

        let text = response.text().await?;
        let rpc_response: RpcResponse<T> = serde_json::from_str(&text)?;

        if let Some(error) = rpc_response.error {
            return Err(Error::Rpc {
                code: error.code,
                message: error.message,
            });
        }

        Ok(rpc_response)
    }
}

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    result: T,
    error: Option<RpcError>,
    #[allow(dead_code)]
    id: u64,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct RpcAccountInfo {
    value: Option<UiAccount>,
}

#[derive(Deserialize)]
struct RpcMultipleAccounts {
    value: Vec<Option<UiAccount>>,
}

/// Fetch a single account from a Solana RPC endpoint.
pub async fn fetch_account(url: &str, pubkey: &Pubkey) -> Result<Option<Account>, Error> {
    let client = RpcClient::new(url);
    client.get_account(pubkey).await
}

/// Fetch multiple accounts from a Solana RPC endpoint.
///
/// Returns exactly one account for each requested pubkey. If an account doesn't
/// exist on-chain, `Account::default()` is used.
pub async fn fetch_accounts(
    url: &str,
    pubkeys: &[Pubkey],
) -> Result<Vec<(Pubkey, Account)>, Error> {
    fetch_accounts_with_default(url, pubkeys, |_| Account::default()).await
}

/// Fetch multiple accounts from a Solana RPC endpoint with a custom default for
/// missing accounts.
///
/// Returns exactly one account for each requested pubkey. If an account doesn't
/// exist on-chain, the provided default function is called.
pub async fn fetch_accounts_with_default<F>(
    url: &str,
    pubkeys: &[Pubkey],
    default_account: F,
) -> Result<Vec<(Pubkey, Account)>, Error>
where
    F: Fn(&Pubkey) -> Account,
{
    let client = RpcClient::new(url);
    let accounts = client.get_multiple_accounts(pubkeys).await?;

    let mut result = Vec::new();
    for (pubkey, account_opt) in pubkeys.iter().zip(accounts.into_iter()) {
        let account = account_opt.unwrap_or_else(|| default_account(pubkey));
        result.push((*pubkey, account));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        solana_sdk_ids::{system_program::ID as SYSTEM_PROGRAM_ID, vote::ID as VOTE_PROGRAM_ID},
    };

    #[tokio::test]
    async fn test_fetch_accounts() {
        let client = RpcClient::new("https://api.mainnet-beta.solana.com");

        // Fetch the system program (should always exist).
        match client.get_account(&SYSTEM_PROGRAM_ID).await {
            Ok(Some(account)) => {
                assert_eq!(account.owner, solana_sdk_ids::native_loader::id());
            }
            Ok(None) => panic!("System program should exist"),
            Err(e) => panic!("Failed to fetch system program: {:?}", e),
        }

        // Fetch a non-existent account.
        let random_pubkey = Pubkey::new_unique();
        let account = client.get_account(&random_pubkey).await.unwrap();
        assert!(account.is_none());
    }

    #[tokio::test]
    async fn test_fetch_multiple_accounts() {
        let client = RpcClient::new("https://api.mainnet-beta.solana.com");

        let random_pubkey = Pubkey::new_unique();
        let accounts = client
            .get_multiple_accounts(&[SYSTEM_PROGRAM_ID, VOTE_PROGRAM_ID, random_pubkey])
            .await
            .unwrap();

        assert_eq!(accounts.len(), 3);
        assert!(accounts[0].is_some()); // System program exists
        assert!(accounts[1].is_some()); // Vote program exists
        assert!(accounts[2].is_none()); // Random account doesn't exist
    }
}
