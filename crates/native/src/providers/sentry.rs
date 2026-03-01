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
/// Designed for MetaMask-style flows where nonces are strictly sequential.
#[cfg(feature = "http")]
pub struct SentryNonceProvider {
    client: Client,
    base_url: String,
}

#[cfg(feature = "http")]
impl SentryNonceProvider {
    /// Creates a new provider pointing to a Sentry node.
    ///
    /// # Errors
    ///
    /// Returns [`SigningError::Nonce`] if the HTTP client cannot be initialized
    /// (e.g., TLS backend failure).
    pub fn new(base_url: impl Into<String>) -> Result<Self, SigningError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| {
                SigningError::Nonce(NonceError::FetchFailed(format!(
                    "failed to build HTTP client for Sentry: {e}"
                )))
            })?;

        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    /// Default for local development (`http://127.0.0.1:8080`).
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client cannot be constructed (should not happen
    /// under normal system conditions).
    pub fn local() -> Self {
        Self::new("http://127.0.0.1:8080")
            .expect("failed to build local Sentry provider")
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
        let url = format!(
            "{}/auth/v1/nonce-state?address={address_hex}",
            self.base_url
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                SigningError::Nonce(NonceError::FetchFailed(format!(
                    "GET {url} failed: {e}"
                )))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SigningError::Nonce(NonceError::FetchFailed(format!(
                "GET {url} returned HTTP {status}: {body}"
            ))));
        }

        let query_resp: QueryNonceStateResponse =
            response.json().await.map_err(|e| {
                SigningError::Nonce(NonceError::FetchFailed(format!(
                    "GET {url} returned invalid JSON: {e}"
                )))
            })?;

        let next_monotonic = query_resp
            .last_monotonic
            .checked_add(1)
            .ok_or_else(|| {
                SigningError::Nonce(NonceError::FetchFailed(
                    "monotonic nonce overflow".to_string(),
                ))
            })?;

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
