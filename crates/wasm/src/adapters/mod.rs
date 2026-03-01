//! Browser wallet adapters for WASM environments.
//!
//! Provides concrete [`Signer`] implementations for the three major injected wallet
//! families, plus a `WasmSigner` dispatch enum used by [`TxBuilderWasm`](crate::bindings::TxBuilderWasm).
//!
//! Each adapter:
//! - Connects eagerly via an `async fn connect()` that populates the cached address/pubkey.
//! - Uses [`RefCell`] for interior mutability of the cached wallet state, enabling
//!   future account-change handling without requiring `&mut self`.
//! - Implements [`Signer`] to integrate seamlessly with the generic [`TxBuilder`].
//!
//! # Safety
//!
//! The adapters use `unsafe impl Send + Sync` because `RefCell` is `!Sync`.
//! This is sound on `wasm32-unknown-unknown` where execution is **guaranteed
//! single-threaded** — no concurrent access to `RefCell` is possible.

pub mod metamask;
pub mod phantom;
pub mod taproot;

pub use metamask::MetaMaskAdapterWasm;
pub use phantom::PhantomAdapterWasm;
pub use taproot::TaprootAdapterWasm;

use async_trait::async_trait;

use morpheum_signing_core::{
    error::SigningError,
    proto::tx::v1::SignDoc,
    signer::Signer,
    types::{AccountId, PublicKey, Signature, WalletType},
};

// ==================== WASM SIGNER DISPATCH ENUM ====================

/// Static-dispatch signer enum for all supported WASM wallet adapters.
///
/// Used by [`TxBuilderWasm`](crate::bindings::TxBuilderWasm) instead of `Box<dyn Signer>`
/// to avoid the need for a blanket `impl Signer for Box<dyn Signer>` and to enable
/// clean, zero-overhead dispatch within the WASM boundary.
pub(crate) enum WasmSigner {
    /// MetaMask / Rabby / EVM injected wallet.
    MetaMask(MetaMaskAdapterWasm),
    /// Phantom / Solflare / Solana injected wallet.
    Phantom(PhantomAdapterWasm),
    /// Unisat / Leather / Xverse — Bitcoin Taproot injected wallet.
    Taproot(TaprootAdapterWasm),
}

// SAFETY: WASM (wasm32-unknown-unknown) is single-threaded by specification.
// No concurrent access is possible, making Send + Sync trivially safe.
unsafe impl Send for WasmSigner {}
unsafe impl Sync for WasmSigner {}

#[async_trait(?Send)]
impl Signer for WasmSigner {
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        match self {
            Self::MetaMask(a) => a.sign_impl(sign_doc).await,
            Self::Phantom(a) => a.sign_impl(sign_doc).await,
            Self::Taproot(a) => a.sign_impl(sign_doc).await,
        }
    }

    fn public_key(&self) -> PublicKey {
        match self {
            Self::MetaMask(a) => a.public_key(),
            Self::Phantom(a) => a.public_key(),
            Self::Taproot(a) => a.public_key(),
        }
    }

    fn wallet_type(&self) -> WalletType {
        match self {
            Self::MetaMask(_) => WalletType::Evm,
            Self::Phantom(_) => WalletType::Solana,
            Self::Taproot(_) => WalletType::Bitcoin,
        }
    }

    fn account_id(&self) -> AccountId {
        match self {
            Self::MetaMask(a) => a.account_id(),
            Self::Phantom(a) => a.account_id(),
            Self::Taproot(a) => a.account_id(),
        }
    }

    fn public_key_proto(&self) -> prost_types::Any {
        match self {
            Self::MetaMask(a) => a.public_key_proto(),
            Self::Phantom(a) => a.public_key_proto(),
            Self::Taproot(a) => a.public_key_proto(),
        }
    }
}
