//! Fundamental `Signer` trait for the morpheum-signing library.
//!
//! This is the core abstraction that **every** signing strategy implements.
//! It is deliberately minimal (Interface Segregation), object-safe, and async
//! to support injected browser wallets, hardware devices, and remote signers.
//!
//! The design follows SOLID, DRY, and best industry patterns (used in solana-sdk,
//! ethers-core, cosmos-sdk-rs, and tower).

use async_trait::async_trait;

use crate::{
    error::SigningError,
    proto::SignDoc,           // from morpheum_primitives::tx::v1
    types::{AccountId, PublicKey, Signature, WalletType},
};

/// Core signing abstraction.
///
/// Every concrete signer (`HumanSigner`, `AgentSigner`, `MetaMaskAdapter`, etc.)
/// implements this trait.
///
/// **Design invariants**:
/// - `sign` receives the exact canonical `SignDoc` protobuf that Morpheum nodes expect.
/// - Returns raw signature bytes (curve-agnostic).
/// - `public_key` and `account_id` are synchronous for performance (used in TxBuilder).
/// - `wallet_type` drives nonce strategy and address mapping in `TxBuilder`.
#[async_trait]
pub trait Signer: Send + Sync + 'static {
    /// Signs the canonical `SignDoc` and returns the raw signature bytes.
    ///
    /// This is the only method that performs cryptographic signing.
    /// The `SignDoc` contains `body_bytes`, `auth_info_bytes`, `chain_id`, and `account_number`
    /// exactly as required by Morpheum's `SIGN_MODE_DIRECT`.
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError>;

    /// Returns the public key of this signer.
    ///
    /// Used by `TxBuilder` to populate `SignerInfo.public_key`.
    fn public_key(&self) -> PublicKey;

    /// Returns the wallet type (used by `TxBuilder` for nonce strategy and mapping).
    fn wallet_type(&self) -> WalletType;

    /// Returns the canonical `AccountId` for this signer.
    ///
    /// Default implementation derives it from the public key (blake3).
    /// Concrete impls may override for efficiency (e.g. cached AgentId).
    fn account_id(&self) -> AccountId {
        self.public_key().to_account_id()
    }
}

/// Convenience type alias for dynamic dispatch (used in collections or complex builders).
pub type BoxedSigner = Box<dyn Signer>;

/// Extension trait that adds common convenience methods while keeping the main trait minimal.
/// (Interface Segregation + DRY)
#[async_trait]
pub trait SignerExt: Signer {
    /// Convenience: signs and returns raw bytes directly (useful for some wallet adapters).
    async fn sign_bytes(&self, sign_doc: &SignDoc) -> Result<Vec<u8>, SigningError> {
        let signature = self.sign(sign_doc).await?;
        Ok(signature.0)
    }
}

// Blanket implementation for all `Signer` implementors
#[async_trait]
impl<T: Signer + ?Sized> SignerExt for T {}