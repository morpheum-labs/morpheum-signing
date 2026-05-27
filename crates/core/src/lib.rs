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

pub mod builder;
pub mod claim;
pub mod error;
pub mod mapper;
pub mod nonce;
pub mod signer;
pub mod types;
pub mod wallet_adapter;

/// Chain-side transaction verification (signature + claim + mapping).
///
/// Requires the `full-crypto` feature for Ed25519 and Secp256k1 verification.
#[cfg(feature = "full-crypto")]
pub mod verifier;

/// Thin bridge to the cryptogram workspace — universal signing, HD derivation,
/// address validation, agent delegation, and EIP-712 support.
///
/// Cryptogram is the single source of truth for all cryptographic operations.
/// This bridge provides clean access without duplicating any logic.
///
/// Requires the `cryptogram` feature.
#[cfg(feature = "cryptogram")]
pub mod cryptogram_bridge;

/// Direct re-export of `cryptogram-crypto` for low-level cryptographic operations.
///
/// Making this an explicit, direct dependency (rather than only transitive through
/// `morpheum-crypto`) improves supply-chain visibility (`cargo audit`, `cargo tree`)
/// and lets advanced consumers import low-level primitives directly:
///
/// ```rust,ignore
/// use morpheum_signing_core::cryptogram_crypto::{ed25519_sign, CryptoError};
/// ```
///
/// For most users, the [`cryptogram_bridge`] module or the [`prelude`] provide
/// a higher-level, more ergonomic interface.
///
/// Requires the `cryptogram` feature.
#[cfg(feature = "cryptogram")]
pub use cryptogram_crypto;

// ==================== PROTO RE-EXPORTS ====================

/// Full protobuf namespace — mirrors the `pb` hierarchy from `morpheum-primitives`.
///
/// Usage:
/// - `proto::tx::v1::Tx`
/// - `proto::market::v1::MsgCreateMarketRequest`
/// - `proto::auth::v1::NonceState`
/// - `proto::identity::v1::AgentId`
pub use morpheum_primitives::pb as proto;

/// Phase 23A semantics-tier re-export so downstream crates (e.g.
/// `morpheum-sdk-core`) can declare a transaction's
/// [`morpheum_primitives::tx_class::TxClass`] via the builder without
/// taking a direct primitives dependency.
pub use morpheum_primitives::tx_class;

/// Prost Any re-export (used heavily in TxBody.messages).
pub use crate::proto::Any;

// ==================== PUBLIC RE-EXPORTS ====================

pub use error::SigningError;

/// Recommended ergonomic prelude for users.
///
/// Brings in core signing types, traits, protobuf definitions, and — when
/// the `cryptogram` feature is enabled — all commonly used cryptographic
/// primitives and standards types via the `morpheum-crypto` meta-crate.
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
        AuthInfo, ModeInfo, Nonce, SignDoc, SignMode, SignerInfo, TransactionType, Tx, TxBody,
        TxRaw,
    };

    pub use super::Any;

    // Traits
    pub use super::builder::TxBuilder;
    pub use super::claim::{TradingKeyClaim, VcClaimBuilder};
    pub use super::mapper::{AddressMapper, DefaultAddressMapper};
    pub use super::nonce::NonceProvider;
    pub use super::signer::Signer;
    pub use super::wallet_adapter::WalletAdapter;

    // Chain-side verifier (feature-gated)
    #[cfg(feature = "full-crypto")]
    pub use super::verifier::{verify_signed_tx, VerifiedTx};

    // Cryptogram: bridge module + unified crypto/types/standards prelude.
    // Enables `SigType`, `ChainType`, `Domain`, `generate_single_from_digest`,
    // `validate_address_for_chain`, `TxSigner`, etc. via a single import.
    #[cfg(feature = "cryptogram")]
    pub use super::cryptogram_bridge;

    #[cfg(feature = "cryptogram")]
    pub use morpheum_crypto::prelude::*;
}

/// Crate version constant (useful for debugging and logging).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
