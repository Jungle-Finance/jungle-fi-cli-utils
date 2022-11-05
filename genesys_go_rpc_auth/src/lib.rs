use anchor_client::solana_client::client_error::reqwest;
use anchor_client::solana_client::client_error::reqwest::Url;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signer;
use solana_sdk::bs58;

/// This message must be signed using an ed25519 key.
pub const SIGNIN_MSG: &str = "Sign in to GenesysGo Shadow Platform.";
pub const SIGNIN_REQUEST_URL: &str = "https://portal.genesysgo.net/api/signin";

/// Metadata included in a response to a successful sign-in request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesysGoUser{
    pub id: u64,
    pub public_key: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Return value of a successful sign-in request.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesysGoAuthResponse {
    pub token: String, // signed and base-58 encoded SIGNIN_MSG
    pub user: GenesysGoUser,
}

/// Signin to GenesysGo, and acquire a bearer token.
pub async fn genesys_go_sign_in(signer: &dyn Signer) -> Result<GenesysGoAuthResponse> {
    let body = GenesysGoAuth::new(signer);
    let client = reqwest::Client::new();
    let resp = client
        .post(Url::parse(SIGNIN_REQUEST_URL).unwrap())
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .send()
        .await?;
    Ok(serde_json::from_str(&resp.text().await?)
        .map_err(|e| anyhow!("Failed to decode response: {e}"))?)
}

/// Sign-in request POST body (JSON),
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesysGoAuth {
    /// Signed, base-58 encoded [SIGNIN_MSG]
    message: String,
    /// Base58 encoded public key of the signer.
    signer: String,
}

impl GenesysGoAuth {
    pub fn new(signer: &dyn Signer) -> Self {
        let signature = signer.sign_message(SIGNIN_MSG.as_bytes());
        Self {
            message: bs58::encode(signature.as_ref()).into_string(),
            signer: signer.pubkey().to_string(),
        }
    }
}

