//! PortalNonceProvider — Hot-path nonce strategy for AI Agents on AgentPortal nodes.
//!
//! Optimized for sub-millisecond performance with TradingKey VC delegation and
//! isolated nonce sub-ranges for unlimited parallelism.

#[cfg(feature = "http")]
use async_trait::async_trait;
#[cfg(feature = "http")]
use reqwest::Client;
#[cfg(feature = "http")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "http")]
use morpheum_signing_core::{
    error::{NonceError, SigningError},
    nonce::NonceProvider,
    proto::tx::v1::Nonce,
    types::AccountId,
};

/// Hot-path nonce provider for AgentPortal (recommended for AI agents, HFT, marketplace).
///
/// This is the primary provider for high-frequency autonomous agents.
/// Connects to a local or remote AgentPortal gRPC/HTTP endpoint.
#[cfg(feature = "http")]
pub struct PortalNonceProvider {
    client: Client,
    base_url: String,
}

#[cfg(feature = "http")]
impl PortalNonceProvider {
    /// Creates a new provider for an AgentPortal instance.
    ///
    /// # Errors
    ///
    /// Returns [`SigningError::Nonce`] if the HTTP client cannot be initialized
    /// (e.g., TLS backend failure).
    pub fn new(base_url: impl Into<String>) -> Result<Self, SigningError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(300))
            .connect_timeout(std::time::Duration::from_millis(100))
            .build()
            .map_err(|e| {
                SigningError::Nonce(NonceError::FetchFailed(format!(
                    "failed to build HTTP client for AgentPortal: {e}"
                )))
            })?;

        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    /// Default for local development (`http://127.0.0.1:9090`).
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client cannot be constructed (should not happen
    /// under normal system conditions).
    pub fn local() -> Self {
        Self::new("http://127.0.0.1:9090")
            .expect("failed to build local AgentPortal provider")
    }
}

/// Request body for the next-nonce endpoint.
#[cfg(feature = "http")]
#[derive(Debug, Serialize)]
struct GenerateNextNonceRequest {
    agent_did: String,
    agent_hash: String,
}

/// Response from AgentPortal containing the canonical `tx.v1.Nonce`.
#[cfg(feature = "http")]
#[derive(Debug, Deserialize)]
struct GenerateNextNonceResponse {
    monotonic: u64,
    ts_ms: u32,
    sub: u32,
}

#[cfg(feature = "http")]
#[async_trait]
impl NonceProvider for PortalNonceProvider {
    async fn next_nonce(&self, account_id: &AccountId) -> Result<Nonce, SigningError> {
        let url = format!("{}/auth/v1/next-nonce", self.base_url);
        let hash = hex::encode(account_id.0);

        let request = GenerateNextNonceRequest {
            agent_did: format!("did:agent:{hash}"),
            agent_hash: hash,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                SigningError::Nonce(NonceError::FetchFailed(format!(
                    "POST {url} failed: {e}"
                )))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SigningError::Nonce(NonceError::FetchFailed(format!(
                "POST {url} returned HTTP {status}: {body}"
            ))));
        }

        let resp: GenerateNextNonceResponse = response.json().await.map_err(|e| {
            SigningError::Nonce(NonceError::FetchFailed(format!(
                "POST {url} returned invalid JSON: {e}"
            )))
        })?;

        Ok(Nonce {
            monotonic: resp.monotonic,
            ts_ms: resp.ts_ms,
            sub: resp.sub,
        })
    }

    fn strategy_name(&self) -> &'static str {
        "agent_portal_hot_nonce_provider"
    }
}
