//! AgentSigner — TradingKey signer for AI agents with VC claim support.
//!
//! This signer is optimized for autonomous agents, HFT, and marketplaces.
//! It uses a dedicated TradingKey (ed25519) and optionally embeds a VC claim
//! for fine-grained delegation and nonce sub-range isolation.

use async_trait::async_trait;
use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use zeroize::{Zeroize, ZeroizeOnDrop};

use morpheum_signing_core::{
    claim::TradingKeyClaim,
    error::SigningError,
    proto::tx::v1::SignDoc,
    signer::Signer,
    types::{AccountId, PublicKey, Signature, WalletType},
};

/// Agent signer using a TradingKey + optional VC claim.
///
/// This is the recommended signer for autonomous AI agents.
#[derive(Debug, Clone)]
pub struct AgentSigner {
    /// The TradingKey used for signing (ed25519).
    trading_key: SigningKey,
    /// The canonical AgentId (from identity registration).
    agent_id: AccountId,
    /// Optional TradingKeyClaim (for delegation, limits, and nonce sub-range).
    claim: Option<TradingKeyClaim>,
}

impl AgentSigner {
    /// Creates a new `AgentSigner` with a TradingKey seed and optional claim.
    ///
    /// In production, the seed should come from secure storage or key derivation.
    #[must_use]
    pub fn new(
        trading_key_seed: &[u8; 32],
        agent_id: AccountId,
        claim: Option<TradingKeyClaim>,
    ) -> Self {
        let trading_key = SigningKey::from_bytes(trading_key_seed);
        Self {
            trading_key,
            agent_id,
            claim,
        }
    }
}

#[async_trait]
impl Signer for AgentSigner {
    /// Signs the canonical `SignDoc` using the TradingKey (ed25519).
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let signature = self.trading_key.sign(&bytes);
        Ok(Signature::Ed25519(signature.to_bytes()))
    }

    /// Returns the public key of the TradingKey.
    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.trading_key.verifying_key().to_bytes())
    }

    /// Returns the wallet type for this signer.
    fn wallet_type(&self) -> WalletType {
        WalletType::Agent
    }

    /// Returns the canonical AgentId (overrides default for efficiency).
    fn account_id(&self) -> AccountId {
        self.agent_id.clone()
    }
}

impl Drop for AgentSigner {
    fn drop(&mut self) {
        self.trading_key.zeroize();
        if let Some(claim) = &mut self.claim {
            claim.signature.0.zeroize();
        }
    }
}

impl ZeroizeOnDrop for AgentSigner {}