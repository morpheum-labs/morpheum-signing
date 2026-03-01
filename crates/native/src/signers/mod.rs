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

mod native;     // ← renamed from human
mod agent;
mod evm;
mod solana;
mod bitcoin;

// Re-exports (short aliases are defined in lib.rs for top-level ergonomics)
pub use native::NativeSigner;
pub use agent::AgentSigner;
pub use evm::EvmSigner;
pub use solana::SolanaSigner;
pub use bitcoin::BitcoinSigner;