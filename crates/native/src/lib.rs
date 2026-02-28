//! Native (std) implementations for the Morpheum Signing SDK.
//!
//! This crate provides concrete, production-ready implementations of the core traits
//! for native environments (CLI, bots, autonomous agents, servers).
//!
//! It re-exports the entire core library and adds:
//! - HumanSigner (local ed25519 keypair, sequential nonce)
//! - AgentSigner (TradingKey + VC claim support)
//! - SentryNonceProvider (HTTP nonce fetching from Sentry nodes)
//! - PortalNonceProvider (hot-path nonce for AgentPortal)
//!
//! All types are ready to use with `TxBuilder`.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

// Re-export the entire core library for seamless use
pub use morpheum_signing_core as core;
pub use core::*;

// Concrete native implementations
mod signers;
mod providers;

// Public re-exports of the most important native types
pub use signers::{HumanSigner, AgentSigner};
pub use providers::{SentryNonceProvider, PortalNonceProvider};

// Convenience aliases for common use cases
pub type Human = HumanSigner;
pub type Agent = AgentSigner;

// Convenience factory functions
pub fn human(signer: HumanSigner) -> TxBuilder<HumanSigner> {
    TxBuilder::human(signer)
}

pub fn agent(signer: AgentSigner) -> TxBuilder<AgentSigner> {
    TxBuilder::agent(signer)
}