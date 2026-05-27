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

mod agent;
mod bitcoin;
mod evm;
mod native; // ← renamed from human
mod solana;

// Re-exports (short aliases are defined in lib.rs for top-level ergonomics)
pub use agent::AgentSigner;
pub use bitcoin::BitcoinSigner;
pub use evm::{EvmSigner, EVM_DEFAULT_PATH};
pub use native::NativeSigner;
pub use solana::{SolanaSigner, SOLANA_DEFAULT_PATH};
