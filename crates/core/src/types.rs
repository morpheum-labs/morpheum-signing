//! Core types for morpheum-signing.
//!
//! This crate is deliberately `no_std` + minimal to support WASM and embedded use.
//! All protobuf types come exclusively from the published morpheum-primitives crate.

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::proto::tx::v1 as tx;

/// Re-export all core protobuf types from primitives (proto-centric).
pub use tx::{
    AuthInfo, ModeInfo, Nonce, SignDoc, SignMode, SignerInfo, Tx, TxBody, TxRaw,
    TransactionType,
};

/// Re-export Any for message packing.
pub use prost_types::Any;

/// Canonical `AccountId` used throughout Morpheum (blake3 hash of address).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Zeroize, ZeroizeOnDrop))]
pub struct AccountId(pub [u8; 32]);

impl AccountId {
    /// Zero-valued `AccountId`.
    pub const ZERO: Self = Self([0u8; 32]);
}

/// Wallet types supported for multi-chain interoperability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WalletType {
    /// Native Morpheum keypair (ed25519).
    Native = 0,
    /// EVM (MetaMask, Ledger, etc.).
    Evm = 1,
    /// Solana (Phantom, Solflare, etc.).
    Solana = 2,
    /// Bitcoin Taproot / Schnorr.
    Bitcoin = 3,
    /// Agent-specific (TradingKey + VC).
    Agent = 4,
    /// Hardware wallet (generic fallback).
    Hardware = 255,
}

impl fmt::Display for WalletType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native => write!(f, "native"),
            Self::Evm => write!(f, "evm"),
            Self::Solana => write!(f, "solana"),
            Self::Bitcoin => write!(f, "bitcoin"),
            Self::Agent => write!(f, "agent"),
            Self::Hardware => write!(f, "hardware"),
        }
    }
}

/// Unified address representation from any chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Address {
    /// Native Morpheum address (morm1...).
    Native(String),
    /// EVM (0x...).
    Evm([u8; 20]),
    /// Solana (base58, 32 bytes).
    Solana([u8; 32]),
    /// Bitcoin Taproot (bc1p...).
    Bitcoin(String),
    /// Agent DID.
    Agent(String),
}

impl Address {
    /// Convert to canonical `AccountId` (blake3 hash) — used by mapper.
    #[must_use]
    pub fn to_account_id(&self) -> AccountId {
        let mut hasher = Sha256::new();
        match self {
            Self::Native(s) | Self::Bitcoin(s) | Self::Agent(s) => hasher.update(s.as_bytes()),
            Self::Evm(bytes) => hasher.update(bytes),
            Self::Solana(bytes) => hasher.update(bytes),
        }
        let hash = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        AccountId(arr)
    }
}

/// Public key wrapper for full multi-curve support.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PublicKey {
    /// Ed25519 32-byte public key (Native + Agent TradingKey).
    Ed25519([u8; 32]),
    /// Secp256k1 compressed public key (33 bytes) — EVM / MetaMask.
    Secp256k1([u8; 33]),
    /// BIP-340 Schnorr X-only public key (32 bytes) — Bitcoin Taproot.
    Schnorr([u8; 32]),
    /// Agent key (alias to Ed25519 for clarity).
    Agent([u8; 32]),
}

impl PublicKey {
    /// Derive the canonical `AccountId` from this public key (blake3 hash).
    #[must_use]
    pub fn to_account_id(&self) -> AccountId {
        let bytes = match self {
            Self::Ed25519(b) | Self::Agent(b) | Self::Schnorr(b) => b.as_slice(),
            Self::Secp256k1(b) => b.as_slice(),
        };
        let hash = Sha256::digest(bytes);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        AccountId(arr)
    }
}

/// Signature bytes for all supported curves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Signature {
    /// Ed25519 64-byte signature.
    Ed25519([u8; 64]),
    /// Secp256k1 64-byte signature (or 65-byte recoverable).
    Secp256k1([u8; 64]),
    /// BIP-340 Schnorr 64-byte signature (Bitcoin Taproot).
    Schnorr([u8; 64]),
}

impl Zeroize for Signature {
    fn zeroize(&mut self) {
        match self {
            Self::Ed25519(b) | Self::Secp256k1(b) | Self::Schnorr(b) => b.zeroize(),
        }
    }
}

impl ZeroizeOnDrop for Signature {}

/// Thin wrapper around a fully signed Morpheum transaction.
#[derive(Debug, Clone)]
pub struct SignedTx {
    /// The canonical broadcast form (decoded).
    pub tx: Tx,
    /// The exact signed bytes that were broadcast (`TxRaw` serialized).
    pub raw_bytes: Vec<u8>,
    /// Optional `TxRaw` for verification/debugging.
    pub tx_raw: Option<TxRaw>,
}

impl SignedTx {
    #[must_use]
    pub const fn new(tx: Tx, raw_bytes: Vec<u8>, tx_raw: Option<TxRaw>) -> Self {
        Self { tx, raw_bytes, tx_raw }
    }

    #[must_use]
    pub const fn tx(&self) -> &Tx {
        &self.tx
    }

    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    #[must_use]
    pub const fn tx_raw(&self) -> Option<&TxRaw> {
        self.tx_raw.as_ref()
    }

    /// Convenience: txhash (sha256 of `raw_bytes`) as hex.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn txhash_hex(&self) -> String {
        let hash = Sha256::digest(&self.raw_bytes);
        hex::encode(hash)
    }
}

/// Helper for building messages as `Any` (used in `TxBuilder`).
pub trait IntoAny: Message {
    fn type_url() -> String;

    fn into_any(self) -> Any
    where
        Self: Sized,
    {
        Any {
            type_url: Self::type_url(),
            value: self.encode_to_vec(),
        }
    }
}

impl<T: Message> IntoAny for T {
    fn type_url() -> String {
        format!("type.googleapis.com/{}", <Self as Message>::NAME)
    }
}

/// Signing options (Morpheum extension).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SigningOptions {
    pub deadline_seconds: Option<u64>,
    pub memo: Option<String>,
    pub include_timestamp: bool,
}

impl SigningOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}