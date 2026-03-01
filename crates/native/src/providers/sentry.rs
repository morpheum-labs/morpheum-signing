//! SentryNonceProvider — Concrete nonce strategy for human/sequential mode.
//!
//! Fetches the nonce state from the real Morpheum endpoint:
//!     GET /auth/v1/nonce-state?address=<hex>
//!
//! Then computes the next canonical `tx.v1.Nonce` for `SIGN_MODE_DIRECT`
//! (sequential monotonic for MetaMask/EVM compatibility).

#[cfg(feature = "http")]
use async_trait::async_trait;
#[cfg(feature = "http")]
use reqwest::Client;
#[cfg(feature = "http")]
use serde::Deserialize;

#[cfg(feature = "http")]
use morpheum_signing_core::{
    error::{NonceError, SigningError},
    nonce::NonceProvider,
    proto::tx::v1::Nonce,
    types::AccountId,
};

/// Concrete nonce provider for Sentry nodes (human / sequential compatibility).
///
/// Uses the real `auth.v1` endpoints from Morpheum.
#[cfg(feature = "http")]
pub struct SentryNonceProvider {
    client: Client,
    base_url: String,
}

#[cfg(feature = "http")]
impl SentryNonceProvider {
    /// Creates a new provider pointing to a Sentry node.
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to build reqwest client");

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

/// Response from `/auth/v1/nonce-state`.
#[cfg(feature = "http")]
#[derive(Debug, Deserialize)]
struct QueryNonceStateResponse {
    last_monotonic: u64,
}

#[cfg(feature = "http")]
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
            .map_err(|_| SigningError::Nonce(NonceError::InvalidResponse))?;

        let next_monotonic = query_resp.last_monotonic + 1;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0);

        Ok(Nonce {
            monotonic: next_monotonic,
            ts_ms: now_ms,
            sub: 0,
        })
    }

    fn strategy_name(&self) -> &'static str {
        "sentry_sequential_nonce_provider"
    }
}
