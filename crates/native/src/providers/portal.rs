//! PortalNonceProvider — Hot-path nonce strategy for AI Agents on AgentPortal nodes.
//!
//! Optimized for sub-millisecond performance with TradingKey VC delegation and
//! isolated nonce sub-ranges for unlimited parallelism.
//!
//! Fully aligned with your real protos: auth.v1, vc.v1, identity.v1, tx.v1.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use morpheum_signing_core::{
    claim::TradingKeyClaim,
    error::{NonceError, SigningError},
    nonce::NonceProvider,
    proto::{
        identity::v1::AgentId,
        tx::v1::Nonce,
    },
    types::AccountId,
};

/// Hot-path nonce provider for AgentPortal (recommended for AI agents, HFT, marketplace).
///
/// This is the primary provider for high-frequency autonomous agents.
#[derive(Debug, Clone)]
pub struct PortalNonceProvider {
    client: Client,
    base_url: String,
}

impl PortalNonceProvider {
    /// Creates a new provider for an AgentPortal instance.
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(300))   // tight for hot-path
            .connect_timeout(std::time::Duration::from_millis(100))
            .build()
            .expect("Failed to build reqwest client for AgentPortal");

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Default for local development.
    pub fn local() -> Self {
        Self::new("http://127.0.0.1:9090")
    }
}

/// Request to AgentPortal for next nonce (supports TradingKey VC from vc.v1).
#[derive(Debug, Serialize)]
struct GenerateNextNonceRequest {
    agent_id: AgentId,
    trading_key_claim: Option<TradingKeyClaim>,
}

/// Response from AgentPortal (contains the canonical tx.v1.Nonce).
#[derive(Debug, Deserialize)]
struct GenerateNextNonceResponse {
    nonce: Nonce,
}

#[async_trait]
impl NonceProvider for PortalNonceProvider {
    async fn next_nonce(&self, account_id: &AccountId) -> Result<Nonce, SigningError> {
        self.next_nonce_with_claim(account_id, None).await
    }

    fn strategy_name(&self) -> &'static str {
        "agent_portal_hot_nonce_provider"
    }
}

impl PortalNonceProvider {
    /// Advanced method: Generate next nonce with TradingKeyClaim for sub-range parallelism.
    pub async fn next_nonce_with_claim(
        &self,
        account_id: &AccountId,
        claim: Option<&TradingKeyClaim>,
    ) -> Result<Nonce, SigningError> {
        let url = format!("{}/auth/v1/next-nonce", self.base_url);

        let request = GenerateNextNonceRequest {
            agent_id: AgentId {
                did: format!("did:agent:{}", hex::encode(account_id.0)),
                hash: hex::encode(account_id.0),
            },
            trading_key_claim: claim.cloned(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| SigningError::Nonce(NonceError::FetchFailed(e.to_string())))?;

        if !response.status().is_success() {
            return Err(SigningError::Nonce(NonceError::FetchFailed(
                format!("HTTP {}", response.status()),
            )));
        }

        let resp: GenerateNextNonceResponse = response
            .json()
            .await
            .map_err(|e| SigningError::Nonce(NonceError::InvalidResponse))?;

        Ok(resp.nonce)
    }
}