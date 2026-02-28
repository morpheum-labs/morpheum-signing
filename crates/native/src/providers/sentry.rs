//! SentryNonceProvider — Concrete nonce strategy for human/sequential mode.
//!
//! Fetches the full NonceState from the real Morpheum endpoint:
//!     GET /auth/v1/nonce-state?address=<hex>
//!
//! Then computes the next canonical tx.v1.Nonce for SIGN_MODE_DIRECT
//! (sequential monotonic for MetaMask/EVM compatibility).

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use morpheum_signing_core::{
    error::{NonceError, SigningError},
    nonce::NonceProvider,
    proto::{auth::v1 as auth, tx::v1::Nonce},
    types::AccountId,
};

/// Concrete nonce provider for Sentry nodes (human / sequential compatibility).
///
/// Uses the real auth.v1 endpoints and structures from your protos.
#[derive(Debug, Clone)]
pub struct SentryNonceProvider {
    client: Client,
    base_url: String,
}

impl SentryNonceProvider {
    /// Creates a new provider pointing to a Sentry node.
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Default for local development.
    pub fn local() -> Self {
        Self::new("http://127.0.0.1:8080")
    }
}

/// Response from /auth/v1/nonce-state (exact match to your proto).
#[derive(Debug, Deserialize)]
struct QueryNonceStateResponse {
    state: auth::NonceState,
}

#[async_trait]
impl NonceProvider for SentryNonceProvider {
    async fn next_nonce(&self, account_id: &AccountId) -> Result<Nonce, SigningError> {
        let address_hex = hex::encode(account_id.0);

        let url = format!("{}/auth/v1/nonce-state?address={}", self.base_url, address_hex);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SigningError::Nonce(NonceError::FetchFailed(e.to_string())))?;

        if !resp.status().is_success() {
            return Err(SigningError::Nonce(NonceError::FetchFailed(
                format!("HTTP {}", resp.status()),
            )));
        }

        let query_resp: QueryNonceStateResponse = resp
            .json()
            .await
            .map_err(|e| SigningError::Nonce(NonceError::InvalidResponse))?;

        let state = query_resp.state;

        // Compute next sequential nonce for human compatibility
        let next_monotonic = state.last_monotonic + 1;

        // Current timestamp in milliseconds (for replay protection)
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0);

        Ok(Nonce {
            monotonic: next_monotonic,
            ts_ms: now_ms,
            sub: 0, // Sequential mode for humans / MetaMask compatibility
        })
    }

    fn strategy_name(&self) -> &'static str {
        "sentry_sequential_nonce_provider"
    }
}