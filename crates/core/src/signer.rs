//! Fundamental `Signer` trait for the morpheum-signing library.
//!
//! This is the core abstraction that **every** signing strategy implements.
//! It is deliberately minimal (Interface Segregation), object-safe, and async
//! to support injected browser wallets, hardware devices, and remote signers.
//!
//! Updated to support the multi-curve `PublicKey` and `Signature` enums from types.rs.
//!
//! On `wasm32` targets, async methods use `?Send` futures (via `async_trait(?Send)`)
//! because browser JS interop types (`JsFuture`, `JsValue`) are inherently `!Send`.
//! This is safe because WASM is single-threaded by specification.

use async_trait::async_trait;

use crate::{
    error::SigningError,
    proto::tx::v1::{self as tx, SignDoc},
    types::{AccountId, PublicKey, Signature, WalletType},
};

/// Core signing abstraction.
///
/// Every concrete signer (`HumanSigner`, `AgentSigner`, `MetaMaskAdapter`, `PhantomAdapter`, etc.)
/// implements this trait.
///
/// **Design invariants**:
/// - `sign` receives the exact canonical `SignDoc` protobuf that Morpheum nodes expect.
/// - Returns the curve-agnostic `Signature` enum (Ed25519, Secp256k1, Schnorr).
/// - `public_key` and `account_id` are synchronous for performance (used in `TxBuilder`).
/// - `wallet_type` drives nonce strategy and address mapping in `TxBuilder`.
///
/// **Security note**: All native `Signer` implementations use constant-time cryptographic
/// operations with respect to secret key material. See individual signer documentation
/// for library-specific guarantees (ed25519-dalek, k256, libsecp256k1).
///
/// **WASM note**: On `wasm32`, futures returned by `sign()` are `!Send` because browser
/// wallet interop (MetaMask, Phantom, Unisat) uses JS Promises that are `!Send`.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait Signer: Send + Sync + 'static {
    /// Signs the canonical `SignDoc` and returns the curve-agnostic `Signature` enum.
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
    /// Default implementation derives it from the public key (blake3 hash).
    /// Concrete impls may override for efficiency (e.g. cached `AgentId`).
    fn account_id(&self) -> AccountId {
        self.public_key().to_account_id()
    }

    /// Returns the public key as a protobuf [`prost_types::Any`] for `SignerInfo.public_key`.
    ///
    /// Default implementation uses [`PublicKey::to_proto_any()`], which encodes
    /// the correct `type_url` and raw key bytes for the chain.
    /// Concrete impls may override for custom proto encoding.
    fn public_key_proto(&self) -> prost_types::Any {
        self.public_key().to_proto_any()
    }

    /// Returns the [`SignMode`](tx::SignMode) for this signer.
    ///
    /// Default implementation derives from [`WalletType::default_sign_mode()`].
    /// Concrete impls may override for non-standard modes (e.g. gasless, EIP-191).
    fn sign_mode(&self) -> tx::SignMode {
        self.wallet_type().default_sign_mode()
    }
}

/// Convenience type alias for dynamic dispatch (used in collections or complex builders).
pub type BoxedSigner = Box<dyn Signer>;

/// Extension trait that adds common convenience methods while keeping the main trait minimal.
/// (Interface Segregation + DRY)
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait SignerExt: Signer {
    /// Convenience: signs and returns raw signature bytes directly (useful for some wallet adapters).
    async fn sign_bytes(&self, sign_doc: &SignDoc) -> Result<Vec<u8>, SigningError> {
        let signature = self.sign(sign_doc).await?;
        Ok(match signature {
            Signature::Ed25519(b) | Signature::Secp256k1(b) | Signature::Schnorr(b) => b.to_vec(),
        })
    }
}

// Blanket implementation for all `Signer` implementors.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl<T: Signer + ?Sized> SignerExt for T {}