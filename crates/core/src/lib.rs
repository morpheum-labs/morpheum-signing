//! Morpheum Signing SDK — Core Library
//!
//! Minimal, `no_std` compatible foundation for universal multi-chain signing.
//! Supports humans (`MetaMask`, Phantom, Taproot, etc.) and AI agents (`TradingKey` + VC claims).
//!
//! This crate is deliberately thin and depends **only** on the published
//! `morpheum-primitives` crate for all protobuf types (Tx, `SignDoc`, Nonce, etc.).
//! No direct dependency on `.proto` files (types come via `morpheum-primitives` → `morpheum-proto`).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, rust_2018_idioms)]
#![allow(clippy::module_name_repetitions)]

extern crate alloc;

// ==================== PUBLIC MODULES ====================

pub mod error;
pub mod types;
pub mod builder;
pub mod claim;
pub mod mapper;
pub mod nonce;
pub mod signer;
pub mod wallet_adapter;

/// Chain-side transaction verification (signature + claim + mapping).
///
/// Requires the `full-crypto` feature for Ed25519 and Secp256k1 verification.
#[cfg(feature = "full-crypto")]
pub mod verifier;

// ==================== PROTO RE-EXPORTS ====================

/// Full protobuf namespace — mirrors the `pb` hierarchy from `morpheum-primitives`.
///
/// Usage:
/// - `proto::tx::v1::Tx`
/// - `proto::market::v1::MsgCreateMarketRequest`
/// - `proto::auth::v1::NonceState`
/// - `proto::identity::v1::AgentId`
pub use morpheum_primitives::pb as proto;

/// Prost Any re-export (used heavily in TxBody.messages).
pub use crate::proto::Any;

// ==================== PUBLIC RE-EXPORTS ====================

pub use error::SigningError;

/// Recommended ergonomic prelude for users.
///
/// ```rust,ignore
/// use morpheum_signing_core::prelude::*;
/// ```
pub mod prelude {
    pub use super::error::SigningError;

    // Core domain types
    pub use super::types::{
        AccountId, Address, IntoAny, PublicKey, Signature, SignedTx, SigningOptions, WalletType,
    };

    // Protobuf types users need most often
    pub use super::proto::tx::v1::{
        AuthInfo, ModeInfo, Nonce, SignDoc, SignMode, SignerInfo, Tx, TxBody, TxRaw,
        TransactionType,
    };

    pub use super::Any;

    // Traits
    pub use super::builder::TxBuilder;
    pub use super::mapper::{AddressMapper, DefaultAddressMapper};
    pub use super::nonce::NonceProvider;
    pub use super::signer::Signer;
    pub use super::wallet_adapter::WalletAdapter;
    pub use super::claim::{TradingKeyClaim, VcClaimBuilder};

    // Chain-side verifier (feature-gated)
    #[cfg(feature = "full-crypto")]
    pub use super::verifier::{verify_signed_tx, VerifiedTx};
}

/// Crate version constant (useful for debugging and logging).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
