use anchor_client::solana_client::client_error::reqwest;
use anchor_client::solana_client::client_error::reqwest::Url;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signer;
use solana_sdk::bs58;

/// This message must be signed using an ed25519 key.
pub const SIGN_IN_MSG: &str = "Sign in to GenesysGo Shadow Platform.";
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
    pub token: String, // Bearer token to include in "Authentication" header.
    pub user: GenesysGoUser,
}

/// Signin to GenesysGo, and acquire a bearer token.
pub async fn genesys_go_sign_in(
    signer: &dyn Signer,
    url: Option<&str>,
) -> Result<GenesysGoAuthResponse> {
    let body = GenesysGoAuth::new(signer);
    let client = reqwest::Client::new();
    let resp = client
        .post(Url::parse(url.unwrap_or(SIGNIN_REQUEST_URL)).unwrap())
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .send()
        .await?;
    Ok(serde_json::from_str(&resp.text().await?)
        .map_err(|e| anyhow!("Failed to decode response: {e}"))?)
}

/// Sign-in request POST body (JSON),
#[derive(Debug, Serialize, Deserialize)]
struct GenesysGoAuth {
    /// Signed, base-58 encoded [SIGNIN_MSG]
    message: String,
    /// Base58 encoded public key of the signer.
    signer: String,
}

impl GenesysGoAuth {
    pub fn new(signer: &dyn Signer) -> Self {
        let signature = signer.sign_message(SIGN_IN_MSG.as_bytes());
        Self {
            message: bs58::encode(signature.as_ref()).into_string(),
            signer: signer.pubkey().to_string(),
        }
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::time::Duration;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::{Keypair, Signature};
    use super::*;
    use axum::{
        routing::post,
        Router,
        extract::Json,
    };
    use tokio::time::sleep;

    async fn mock_sign_in(Json(payload): Json<GenesysGoAuth>) -> Json<GenesysGoAuthResponse> {
        // Deserialize the strings from the request
        let signature = Signature::from_str(&payload.message)
            .unwrap();
        let signer = Pubkey::from_str(&payload.signer).unwrap();
        // Verify the signature against the sign-in message
        assert!(signature.verify(signer.as_ref(), SIGN_IN_MSG.as_ref()));
        Json(GenesysGoAuthResponse {
            token: "token".to_string(),
            user: GenesysGoUser {
                id: 42,
                public_key: signer.to_string(),
                created_at: "12345678".to_string(),
                updated_at: "123456789".to_string(),
            }
        })
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_sign_in() {
        let keypair = Keypair::new();
        tokio::spawn(async move {
            let router = Router::new().route("/", post(mock_sign_in));
            axum::Server::bind(&"127.0.0.1:3333".parse().unwrap())
                .serve(router.into_make_service())
                .await
                .unwrap();
        });
        sleep(Duration::from_secs(1)).await; // wait for server to start
        let response = genesys_go_sign_in(
            &keypair,
            Some("http://127.0.0.1:3333")
        ).await.unwrap();
        assert_eq!(response.token, "token");
        assert_eq!(response.user.id, 42);
        assert_eq!(response.user.public_key, keypair.pubkey().to_string());
        assert_eq!(response.user.created_at, "12345678");
        assert_eq!(response.user.updated_at, "123456789");
    }
}
