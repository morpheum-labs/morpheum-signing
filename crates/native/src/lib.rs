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

mod signers;
mod providers;

// Browser wallet adapters are WASM-only (js_sys / wasm_bindgen interop).
#[cfg(target_arch = "wasm32")]
mod adapters;

// ==================== SIGNERS (Local Keypairs) ====================

pub use signers::{
    NativeSigner,
    AgentSigner,
    EvmSigner,
    EVM_DEFAULT_PATH,
    SolanaSigner,
    SOLANA_DEFAULT_PATH,
    BitcoinSigner,
};

/// Short alias for [`NativeSigner`].
pub type Native = NativeSigner;
/// Short alias for [`AgentSigner`].
pub type Agent = AgentSigner;
/// Short alias for [`EvmSigner`].
pub type Evm = EvmSigner;
/// Short alias for [`SolanaSigner`].
pub type Solana = SolanaSigner;
/// Short alias for [`BitcoinSigner`].
pub type Bitcoin = BitcoinSigner;

// ==================== ADAPTERS (Injected Wallets — WASM only) ====================

#[cfg(target_arch = "wasm32")]
pub use adapters::{
    MetaMaskAdapter,
    PhantomAdapter,
    TaprootAdapter,
};

#[cfg(target_arch = "wasm32")]
pub type MetaMask = MetaMaskAdapter;
#[cfg(target_arch = "wasm32")]
pub type Phantom = PhantomAdapter;
#[cfg(target_arch = "wasm32")]
pub type Taproot = TaprootAdapter;

// ==================== PROVIDERS (Nonce Strategies) ====================

#[cfg(feature = "http")]
pub use providers::{
    SentryNonceProvider,
    PortalNonceProvider,
};

/// Short alias for [`SentryNonceProvider`].
#[cfg(feature = "http")]
pub type Sentry = SentryNonceProvider;

/// Short alias for [`PortalNonceProvider`].
#[cfg(feature = "http")]
pub type Portal = PortalNonceProvider;

// ==================== CONVENIENCE BUILDER FUNCTIONS ====================

/// Creates a `TxBuilder` backed by the **native** Morpheum signer (ed25519).
pub fn native(signer: NativeSigner) -> builder::TxBuilder<NativeSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by an **agent** signer (TradingKey + VC claim).
pub fn agent(signer: AgentSigner) -> builder::TxBuilder<AgentSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by a local **EVM** signer (secp256k1).
#[cfg(feature = "evm")]
pub fn evm(signer: EvmSigner) -> builder::TxBuilder<EvmSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by a local **Solana** signer.
#[cfg(feature = "solana")]
pub fn solana(signer: SolanaSigner) -> builder::TxBuilder<SolanaSigner> {
    builder::TxBuilder::new(signer)
}

/// Creates a `TxBuilder` backed by a local **Bitcoin Taproot** signer (BIP-340 Schnorr).
#[cfg(feature = "bitcoin")]
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

    // Signers
    pub use super::{
        NativeSigner, AgentSigner, EvmSigner, SolanaSigner, BitcoinSigner,
        Native, Agent, Evm, Solana, Bitcoin,
    };

    // Adapters (WASM only)
    #[cfg(target_arch = "wasm32")]
    pub use super::{
        MetaMaskAdapter, PhantomAdapter, TaprootAdapter,
        MetaMask, Phantom, Taproot,
    };

    // Providers (http feature)
    #[cfg(feature = "http")]
    pub use super::{SentryNonceProvider, PortalNonceProvider, Sentry, Portal};

    // Convenience builder functions
    pub use super::{native, agent};
    #[cfg(feature = "evm")]
    pub use super::evm;
    #[cfg(feature = "solana")]
    pub use super::solana;
    #[cfg(feature = "bitcoin")]
    pub use super::bitcoin;

    // Cryptogram bridge
    #[cfg(feature = "cryptogram")]
    pub use super::cryptogram_bridge;
}