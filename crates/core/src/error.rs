//! Error types for the morpheum-signing library.
//! Designed for maximum clarity, no_std compatibility, and production use.
//! All errors are unified under a single `SigningError` for clean APIs.

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;
#[cfg(feature = "std")]
use std::error::Error as StdError;

use thiserror::Error;

/// Top-level error type for the entire signing SDK.
/// Every public API returns `Result<T, SigningError>`.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SigningError {
    /// Cryptographic operation failed (key generation, signing, verification).
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),

    /// Failed to parse or validate a mnemonic, private key, or seed phrase.
    #[error("invalid key or mnemonic: {0}")]
    InvalidKey(String),

    /// Injected wallet (MetaMask, Phantom, etc.) rejected the request or failed to respond.
    #[error("wallet adapter rejected or failed: {0}")]
    WalletAdapter(String),

    /// Nonce provider (Sentry or AgentPortal) failed to return a valid nonce.
    #[error("nonce provider failed: {0}")]
    Nonce(#[from] NonceError),

    /// Failed to map an external chain address (0x..., sol..., bc1p...) to a Morpheum AccountId.
    #[error("address mapping failed for '{address}': {reason}")]
    AddressMapping {
        address: String,
        reason: String,
    },

    /// Failed to serialize or deserialize protobuf messages (Tx, SignDoc, Any, etc.).
    #[error("protobuf error: {0}")]
    Proto(#[from] prost::EncodeError), // also covers DecodeError via From

    /// VC / TradingKey claim is invalid or expired.
    #[error("invalid VC or TradingKey claim: {0}")]
    InvalidClaim(String),

    /// General signing operation failed (e.g., payload too large, unsupported mode).
    #[error("signing failed: {0}")]
    Signing(String),

    /// I/O error (only available with `std` feature — e.g., file-based key loading).
    #[cfg(feature = "std")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error for rare cases (used internally or by extensions).
    #[error("custom error: {0}")]
    Custom(String),
}

/// Specialized crypto errors (ed25519, secp256k1, etc.).
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CryptoError {
    #[error("ed25519 error: {0}")]
    Ed25519(String),

    #[error("secp256k1 error: {0}")]
    Secp256k1(String),

    #[error("invalid public key length")]
    InvalidPublicKeyLength,

    #[error("signature verification failed")]
    SignatureVerificationFailed,

    #[error("unsupported curve for this operation")]
    UnsupportedCurve,
}

/// Specialized nonce-related errors.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum NonceError {
    #[error("failed to fetch nonce from node: {0}")]
    FetchFailed(String),

    #[error("invalid nonce response from node")]
    InvalidResponse,

    #[error("nonce too old or replay detected")]
    ReplayDetected,
}

// Automatic std::error::Error impl when std is enabled
#[cfg(feature = "std")]
impl StdError for SigningError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Crypto(e) => Some(e),
            Self::Nonce(e) => Some(e),
            Self::Proto(e) => Some(e),
            #[cfg(feature = "std")]
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl StdError for CryptoError {}
#[cfg(feature = "std")]
impl StdError for NonceError {}

// Convenience constructors (keeps code DRY)
impl SigningError {
    pub fn invalid_key(msg: impl Into<String>) -> Self {
        Self::InvalidKey(msg.into())
    }

    pub fn wallet_adapter(msg: impl Into<String>) -> Self {
        Self::WalletAdapter(msg.into())
    }

    pub fn signing(msg: impl Into<String>) -> Self {
        Self::Signing(msg.into())
    }

    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

impl From<prost::DecodeError> for SigningError {
    fn from(err: prost::DecodeError) -> Self {
        Self::Proto(prost::EncodeError::from(err)) // reuse via From<EncodeError> chain
    }
}