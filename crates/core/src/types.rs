//! Core types for morpheum-signing.
//! This crate is deliberately no_std + minimal to support WASM and embedded use.
//! All protobuf types come exclusively from the published morpheum-primitives crate.

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;
use morpheum_primitives::tx::v1 as proto;
use prost::Message;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Re-export all core protobuf types from primitives (proto-centric)
pub use proto::{
    AuthInfo, ModeInfo, Nonce, SignDoc, SignerInfo, Tx, TxBody, TxRaw,
    TransactionType, SignMode,
    // Morpheum extensions already present in primitives
};

/// Re-export Any for message packing
pub use prost_types::Any;

/// Canonical AccountId used throughout Morpheum (blake3 hash of address)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Zeroize, ZeroizeOnDrop))]
pub struct AccountId(pub [u8; 32]);

impl AccountId {
    pub const ZERO: Self = Self([0u8; 32]);
}

/// Wallet types supported for multi-chain interoperability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WalletType {
    /// Native Morpheum keypair (ed25519)
    Native = 0,
    /// EVM (MetaMask, Ledger, etc.)
    Evm = 1,
    /// Solana (Phantom, Solflare, etc.)
    Solana = 2,
    /// Bitcoin Taproot / Schnorr
    Bitcoin = 3,
    /// Agent-specific (TradingKey + VC)
    Agent = 4,
    /// Hardware wallet (generic fallback)
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

/// Unified address representation from any chain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Address {
    /// Native Morpheum address (morm1...)
    Native(String),
    /// EVM (0x...)
    Evm([u8; 20]),
    /// Solana (base58)
    Solana([u8; 32]),
    /// Bitcoin Taproot (bc1p...)
    Bitcoin(String),
    /// Agent DID
    Agent(String),
}

impl Address {
    /// Convert to canonical AccountId (blake3 hash) — used by mapper
    pub fn to_account_id(&self) -> AccountId {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        match self {
            Address::Native(s) => hasher.update(s.as_bytes()),
            Address::Evm(bytes) => hasher.update(bytes),
            Address::Solana(bytes) => hasher.update(bytes),
            Address::Bitcoin(s) => hasher.update(s.as_bytes()),
            Address::Agent(s) => hasher.update(s.as_bytes()),
        }
        let hash = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        AccountId(arr)
    }
}

/// Thin wrapper around a fully signed Morpheum transaction.
/// This is what users receive from `.sign()` and what they send to nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTx {
    /// The canonical broadcast form
    pub tx: Tx,
    /// The exact signed bytes that were broadcast (TxRaw serialized)
    pub raw_bytes: Vec<u8>,
    /// Optional TxRaw for verification/debugging
    pub tx_raw: Option<TxRaw>,
}

impl SignedTx {
    /// Create from a fully built Tx (used internally by builder)
    pub fn new(tx: Tx, raw_bytes: Vec<u8>, tx_raw: Option<TxRaw>) -> Self {
        Self { tx, raw_bytes, tx_raw }
    }

    /// Convenience: txhash (sha256 of raw_bytes) as hex
    #[cfg(feature = "std")]
    pub fn txhash_hex(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.raw_bytes);
        hex::encode(hasher.finalize())
    }
}

/// Public key wrapper for multi-curve support
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PublicKey {
    Ed25519([u8; 32]),
    Secp256k1([u8; 33]),
    Agent([u8; 32]), // same as Ed25519 for TradingKey
}

/// Signature bytes (curve-agnostic)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Signature(pub Vec<u8>);

impl Zeroize for Signature {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for Signature {}

/// Helper for building messages as Any (used in TxBuilder)
pub trait IntoAny: Message + Default {
    fn into_any(self) -> Any {
        Any {
            type_url: format!("type.googleapis.com/{}", self.descriptor().full_name()),
            value: self.encode_to_vec(),
        }
    }
}

// Blanket impl for all prost Messages
impl<T: Message + Default> IntoAny for T {}

/// Signing options (Morpheum extension)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SigningOptions {
    pub deadline_seconds: Option<u64>,
    pub memo: Option<String>,
    pub include_timestamp: bool,
}

impl SigningOptions {
    pub fn new() -> Self {
        Self::default()
    }
}