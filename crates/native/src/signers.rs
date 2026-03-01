//! Concrete signers for native environments (CLI, bots, autonomous agents).
//!
//! - HumanSigner: local ed25519 keypair (MetaMask-style sequential nonce)
//! - AgentSigner: TradingKey + VC claim support (unlimited parallelism)
//! - EvmSigner: local secp256k1 (for full local EVM signing if needed)

use async_trait::async_trait;
use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use k256::ecdsa::{SigningKey as SecpSigningKey, VerifyingKey as SecpVerifyingKey, Signature as SecpSignature};
use zeroize::{Zeroize, ZeroizeOnDrop};

use morpheum_signing_core::{
    claim::TradingKeyClaim,
    error::SigningError,
    proto::SignDoc,
    signer::Signer,
    types::{AccountId, PublicKey, Signature, WalletType},
};

// ==================== HUMAN SIGNER (ed25519) ====================

#[derive(Debug, Clone)]
pub struct HumanSigner {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl HumanSigner {
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self { signing_key, verifying_key }
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

// ==================== AGENT SIGNER (ed25519 TradingKey) ====================

#[derive(Debug, Clone)]
pub struct AgentSigner {
    trading_key: SigningKey,
    agent_id: AccountId,
    claim: Option<TradingKeyClaim>,
}

impl AgentSigner {
    pub fn new(trading_key_seed: &[u8; 32], agent_id: AccountId, claim: Option<TradingKeyClaim>) -> Self {
        let trading_key = SigningKey::from_bytes(trading_key_seed);
        Self { trading_key, agent_id, claim }
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
            claim.signature.0.zeroize();
        }
    }
}

// ==================== EVM SIGNER (secp256k1) ====================

#[derive(Debug, Clone)]
pub struct EvmSigner {
    signing_key: SecpSigningKey,
    verifying_key: SecpVerifyingKey,
}

impl EvmSigner {
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SecpSigningKey::from_slice(seed).expect("Invalid secp256k1 seed");
        let verifying_key = signing_key.verifying_key();
        Self { signing_key, verifying_key }
    }
}

#[async_trait]
impl Signer for EvmSigner {
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let (signature, _) = self.signing_key.sign_prehash_recoverable(&bytes)
            .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(e.to_string())))?;
        Ok(Signature(signature.to_bytes().to_vec()))
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Secp256k1(self.verifying_key.to_encoded_point(true).as_bytes().try_into().unwrap())
    }

    fn wallet_type(&self) -> WalletType {
        WalletType::Evm
    }
}

impl Drop for EvmSigner {
    fn drop(&mut self) {
        // k256 handles zeroization internally in practice
    }
}