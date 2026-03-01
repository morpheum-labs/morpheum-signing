//! Error types for the morpheum-signing library.
//!
//! Designed for maximum clarity and production use.
//! All errors are unified under a single `SigningError` for clean APIs.

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

    /// Injected wallet (`MetaMask`, Phantom, etc.) rejected the request or failed to respond.
    #[error("wallet adapter rejected or failed: {0}")]
    WalletAdapter(String),

    /// Nonce provider (Sentry or `AgentPortal`) failed to return a valid nonce.
    #[error("nonce provider failed: {0}")]
    Nonce(#[from] NonceError),

    /// Failed to map an external chain address (0x..., sol..., bc1p...) to a Morpheum `AccountId`.
    #[error("address mapping failed for '{address}': {reason}")]
    AddressMapping {
        /// The address that failed to map.
        address: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// Failed to encode a protobuf message.
    #[error("protobuf encode error: {0}")]
    ProtoEncode(#[from] prost::EncodeError),

    /// Failed to decode a protobuf message.
    #[error("protobuf decode error: {0}")]
    ProtoDecode(#[from] prost::DecodeError),

    /// VC / `TradingKey` claim is invalid or expired.
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
    /// Ed25519 operation failed.
    #[error("ed25519 error: {0}")]
    Ed25519(String),

    /// Secp256k1 operation failed.
    #[error("secp256k1 error: {0}")]
    Secp256k1(String),

    /// Public key has an invalid length.
    #[error("invalid public key length")]
    InvalidPublicKeyLength,

    /// Signature verification did not succeed.
    #[error("signature verification failed")]
    SignatureVerificationFailed,

    /// The curve is not supported for this operation.
    #[error("unsupported curve for this operation")]
    UnsupportedCurve,
}

/// Specialized nonce-related errors.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum NonceError {
    /// Network or service call to fetch nonce failed.
    #[error("failed to fetch nonce from node: {0}")]
    FetchFailed(String),

    /// Response from the nonce provider was malformed.
    #[error("invalid nonce response from node")]
    InvalidResponse,

    /// Nonce was already consumed or too old (replay attempt).
    #[error("nonce too old or replay detected")]
    ReplayDetected,
}

// ==================== CONVENIENCE CONSTRUCTORS ====================

impl SigningError {
    /// Create an `InvalidKey` error.
    pub fn invalid_key(msg: impl Into<String>) -> Self {
        Self::InvalidKey(msg.into())
    }

    /// Create a `WalletAdapter` error.
    pub fn wallet_adapter(msg: impl Into<String>) -> Self {
        Self::WalletAdapter(msg.into())
    }

    /// Create a `Signing` error.
    pub fn signing(msg: impl Into<String>) -> Self {
        Self::Signing(msg.into())
    }

    /// Create a `Custom` error.
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }

    /// Create an `InvalidClaim` error.
    pub fn invalid_claim(msg: impl Into<String>) -> Self {
        Self::InvalidClaim(msg.into())
    }
}
