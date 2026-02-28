//! Morpheum Signing SDK — Core Library
//!
//! Minimal, `no_std` compatible foundation for universal multi-chain signing.
//! Supports humans (MetaMask, Phantom, Taproot, etc.) and AI agents (TradingKey + VC claims).
//!
//! This crate is deliberately thin and depends **only** on the published
//! `morpheum-primitives` crate for all protobuf types (Tx, SignDoc, Nonce, etc.).
//! No direct dependency on proto-lib or any .proto files.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms
)]
#![allow(clippy::module_name_repetitions)]

extern crate alloc;

// ==================== PUBLIC MODULES ====================

// Currently implemented
pub mod error;
pub mod types;

// Future modules (declared here for architectural clarity and to prevent
// any future breaking changes when files are added in strict order)
pub mod builder;
pub mod claim;
pub mod mapper;
pub mod nonce;
pub mod signer;
pub mod wallet_adapter;

// ==================== PUBLIC RE-EXPORTS ====================

// Most commonly used items at crate root for convenience
pub use error::SigningError;
pub use types::*;

// Full protobuf namespace for easy message construction (e.g. MsgCreateMarketRequest)
pub use morpheum_primitives::tx::v1 as proto;

// Prost Any re-export (used heavily in TxBody.messages)
pub use prost_types::Any;

/// Recommended ergonomic prelude for users.
///
/// ```rust
/// use morpheum_signing_core::prelude::*;
/// ```
pub mod prelude {
    pub use super::error::SigningError;

    // Core domain types
    pub use super::types::{
        AccountId,
        Address,
        PublicKey,
        Signature,
        SignedTx,
        WalletType,
        SigningOptions,
    };

    // Protobuf types users need most often
    pub use super::proto::{
        AuthInfo,
        ModeInfo,
        Nonce,
        SignDoc,
        SignerInfo,
        Tx,
        TxBody,
        TxRaw,
        TransactionType,
        SignMode,
    };

    pub use super::Any;

    // Will be populated as modules are implemented (no compile errors until then)
    // pub use super::builder::TxBuilder;
    // pub use super::signer::Signer;
    // pub use super::wallet_adapter::WalletAdapter;
    // pub use super::nonce::NonceProvider;
    // pub use super::mapper::AddressMapper;
    // pub use super::claim::VcClaimBuilder;
}

// Crate version constant (useful for debugging and logging)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Convenience re-export of the prelude at crate root
pub use prelude::*;