//! Concrete signers for native environments (CLI, bots, autonomous agents).
//!
//! - HumanSigner: local ed25519 keypair (MetaMask-style sequential nonce)
//! - AgentSigner: TradingKey + VC claim support (unlimited parallelism)

use async_trait::async_trait;
use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use zeroize::{Zeroize, ZeroizeOnDrop};

use morpheum_signing_core::{
    claim::TradingKeyClaim,
    error::SigningError,
    proto::SignDoc,
    signer::Signer,
    types::{AccountId, PublicKey, Signature, WalletType},
};

/// Local ed25519 signer for humans (MetaMask / EVM compatibility).
///
/// Uses sequential nonce by default (via SentryNonceProvider).
#[derive(Debug, Clone)]
pub struct HumanSigner {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl HumanSigner {
    /// Create from a 32-byte seed (recommended: use a secure RNG or mnemonic).
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create from a mnemonic (BIP-39) — convenience for CLI/human use.
    /// Requires `bip39` feature in a future extension.
    pub fn from_mnemonic(_mnemonic: &str) -> Result<Self, SigningError> {
        // Placeholder — in full production add bip39 crate
        Err(SigningError::invalid_key("mnemonic support requires extra feature"))
    }
}

#[async_trait]
impl Signer for HumanSigner {
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let signature = self.signing_key.sign(&bytes);
        Ok(Signature(signature.to_bytes().to_vec()))
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.verifying_key.to_bytes())
    }

    fn wallet_type(&self) -> WalletType {
        WalletType::Native
    }
}

impl Drop for HumanSigner {
    fn drop(&mut self) {
        self.signing_key.zeroize();
    }
}

/// Agent-specific signer using a TradingKey + optional VC claim.
///
/// This is the recommended signer for autonomous AI agents, HFT, and marketplaces.
#[derive(Debug, Clone)]
pub struct AgentSigner {
    trading_key: SigningKey,
    agent_id: AccountId,
    claim: Option<TradingKeyClaim>,
}

impl AgentSigner {
    /// Create a new agent signer with a TradingKey.
    pub fn new(trading_key_seed: &[u8; 32], agent_id: AccountId, claim: Option<TradingKeyClaim>) -> Self {
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
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let signature = self.trading_key.sign(&bytes);
        Ok(Signature(signature.to_bytes().to_vec()))
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.trading_key.verifying_key().to_bytes())
    }

    fn wallet_type(&self) -> WalletType {
        WalletType::Agent
    }

    fn account_id(&self) -> AccountId {
        self.agent_id.clone()
    }
}

impl Drop for AgentSigner {
    fn drop(&mut self) {
        self.trading_key.zeroize();
        if let Some(claim) = &mut self.claim {
            // Claim contains signature — zeroize it
            claim.signature.0.zeroize();
        }
    }
}