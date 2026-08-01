//! Native (std) implementations for the Morpheum Signing SDK.
//!
//! This crate provides concrete, production-ready implementations of the core traits
//! for native environments (CLI tools, bots, autonomous agents, servers).
//!
//! # Main Components
//! - **Signers** (`signers/`): Local keypair implementations
//!   - `NativeSigner` — Morpheum native ed25519 signer (recommended for most humans)
//!   - `AgentSigner` — TradingKey + VC claim signer for autonomous agents
//!   - `EvmSigner` — secp256k1 for EVM compatibility
//!   - `SolanaSigner` — ed25519 for Solana compatibility
//!   - `BitcoinSigner` — BIP-340 Schnorr for Bitcoin Taproot
//! - **Adapters** (`adapters/`): Injected wallet support (MetaMask, Phantom, Taproot)
//! - **Providers** (`providers/`): Nonce strategies (Sentry + Portal)
//!
//! All types integrate seamlessly with [`TxBuilder`](morpheum_signing_core::builder::TxBuilder).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

// Re-export the entire core library for seamless use
pub use morpheum_signing_core as core;
pub use morpheum_signing_core::*;

// ==================== MODULES ====================

mod providers;
mod signers;

// Browser wallet adapters are WASM-only (js_sys / wasm_bindgen interop).
#[cfg(target_arch = "wasm32")]
mod adapters;

// ==================== SIGNERS (Local Keypairs) ====================

// Re-exported per backend, matching the gates in `signers`: a build that
// does not enable a backend does not link its dependency, so the signer
// it powers cannot exist to be re-exported.
#[cfg(feature = "schnorr")]
pub use signers::BitcoinSigner;
#[cfg(feature = "ed25519")]
pub use signers::{AgentSigner, NativeSigner, SolanaSigner, SOLANA_DEFAULT_PATH};
#[cfg(feature = "secp256k1")]
pub use signers::{EvmSigner, EVM_DEFAULT_PATH};

/// Short alias for [`NativeSigner`].
#[cfg(feature = "ed25519")]
pub type Native = NativeSigner;
/// Short alias for [`AgentSigner`].
#[cfg(feature = "ed25519")]
pub type Agent = AgentSigner;
/// Short alias for [`EvmSigner`].
#[cfg(feature = "secp256k1")]
pub type Evm = EvmSigner;
/// Short alias for [`SolanaSigner`].
#[cfg(feature = "ed25519")]
pub type Solana = SolanaSigner;
/// Short alias for [`BitcoinSigner`].
#[cfg(feature = "schnorr")]
pub type Bitcoin = BitcoinSigner;

// ==================== ADAPTERS (Injected Wallets — WASM only) ====================

#[cfg(target_arch = "wasm32")]
pub use adapters::{MetaMaskAdapter, PhantomAdapter, TaprootAdapter};

#[cfg(target_arch = "wasm32")]
pub type MetaMask = MetaMaskAdapter;
#[cfg(target_arch = "wasm32")]
pub type Phantom = PhantomAdapter;
#[cfg(target_arch = "wasm32")]
pub type Taproot = TaprootAdapter;

// ==================== PROVIDERS (Nonce Strategies) ====================

#[cfg(feature = "http")]
pub use providers::{PortalNonceProvider, SentryNonceProvider};

/// Short alias for [`SentryNonceProvider`].
#[cfg(feature = "http")]
pub type Sentry = SentryNonceProvider;

/// Short alias for [`PortalNonceProvider`].
#[cfg(feature = "http")]
pub type Portal = PortalNonceProvider;

// ==================== CONVENIENCE BUILDER FUNCTIONS ====================

/// Creates a `TxBuilder` backed by the **native** Morpheum signer (ed25519).
#[cfg(feature = "ed25519")]
pub fn native(signer: NativeSigner) -> builder::TxBuilder<NativeSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by an **agent** signer (TradingKey + VC claim).
#[cfg(feature = "ed25519")]
pub fn agent(signer: AgentSigner) -> builder::TxBuilder<AgentSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by a local **EVM** signer (secp256k1).
#[cfg(feature = "secp256k1")]
pub fn evm(signer: EvmSigner) -> builder::TxBuilder<EvmSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by a local **Solana** signer.
#[cfg(feature = "ed25519")]
pub fn solana(signer: SolanaSigner) -> builder::TxBuilder<SolanaSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by a local **Bitcoin Taproot** signer (BIP-340 Schnorr).
#[cfg(feature = "schnorr")]
pub fn bitcoin(signer: BitcoinSigner) -> builder::TxBuilder<BitcoinSigner> {
    builder::TxBuilder::new(signer)
}

// ==================== CRYPTOGRAM BRIDGE (Feature-gated) ====================

/// Re-export the cryptogram bridge for native consumers.
///
/// Provides universal signing, HD derivation, address validation,
/// agent delegation, and EIP-712 support — all backed by the
/// cryptogram workspace as the single source of truth.
#[cfg(feature = "cryptogram")]
pub use morpheum_signing_core::cryptogram_bridge;

// ==================== RECOMMENDED PRELUDE ====================

/// Recommended prelude for native usage.
///
/// ```rust
/// use morpheum_signing_native::prelude::*;
/// ```
pub mod prelude {
    pub use super::core::prelude::*;

    // Signers, per backend — mirrors the gates on the re-exports above.
    #[cfg(feature = "ed25519")]
    pub use super::{Agent, AgentSigner, Native, NativeSigner, Solana, SolanaSigner};
    #[cfg(feature = "schnorr")]
    pub use super::{Bitcoin, BitcoinSigner};
    #[cfg(feature = "secp256k1")]
    pub use super::{Evm, EvmSigner};

    // Adapters (WASM only)
    #[cfg(target_arch = "wasm32")]
    pub use super::{MetaMask, MetaMaskAdapter, Phantom, PhantomAdapter, Taproot, TaprootAdapter};

    // Providers (http feature)
    #[cfg(feature = "http")]
    pub use super::{Portal, PortalNonceProvider, Sentry, SentryNonceProvider};

    // Convenience builder functions, gated to match their definitions.
    #[cfg(feature = "schnorr")]
    pub use super::bitcoin;
    #[cfg(feature = "secp256k1")]
    pub use super::evm;
    #[cfg(feature = "ed25519")]
    pub use super::{agent, native, solana};

    // Cryptogram bridge
    #[cfg(feature = "cryptogram")]
    pub use super::cryptogram_bridge;
}
