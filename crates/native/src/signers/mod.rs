//! Signers submodule — Local keypair signers for native environments.
//!
//! This module contains concrete implementations of the `Signer` trait for:
//! - NativeSigner (ed25519)
//! - AgentSigner (TradingKey + VC claims)
//! - EvmSigner (secp256k1)
//! - SolanaSigner (ed25519 for Solana)
//! - BitcoinSigner (BIP-340 Schnorr for Taproot)
//!
//! All signers are re-exported at this level for ergonomic use.
//!
//! Each module is gated on the backend feature that supplies its crypto
//! dependency — `ed25519` for dalek, `secp256k1` for k256, `schnorr` for
//! the bitcoin crate. Those features already existed; the modules simply
//! never declared which one they needed, so a build without them
//! compiled every signer against dependencies that were not linked.

#[cfg(feature = "ed25519")]
mod agent;
#[cfg(feature = "schnorr")]
mod bitcoin;
#[cfg(feature = "secp256k1")]
mod evm;
#[cfg(feature = "ed25519")]
mod native; // ← renamed from human
#[cfg(feature = "ed25519")]
mod solana;

// Re-exports (short aliases are defined in lib.rs for top-level ergonomics)
#[cfg(feature = "ed25519")]
pub use agent::AgentSigner;
#[cfg(feature = "schnorr")]
pub use bitcoin::BitcoinSigner;
#[cfg(feature = "secp256k1")]
pub use evm::{EvmSigner, EVM_DEFAULT_PATH};
#[cfg(feature = "ed25519")]
pub use native::NativeSigner;
#[cfg(feature = "ed25519")]
pub use solana::{SolanaSigner, SOLANA_DEFAULT_PATH};
