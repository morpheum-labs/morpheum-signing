//! Thin bridge between `morpheum-signing` and the `cryptogram` workspace.
//!
//! Cryptogram is the **single source of truth** for cryptographic primitives,
//! universal signing, HD key derivation, address validation, agent delegation,
//! and EIP-712 typed data signing.
//!
//! This module provides:
//! - **Type conversions** between signing SDK and cryptogram types.
//! - **Signing delegation** to cryptogram-crypto's universal signing engine.
//! - **Verification delegation** for multi-algorithm, multi-chain signatures.
//! - **Address validation** using cryptogram's chain-type-aware validators.
//! - **Re-exports** of essential cryptogram types for downstream consumers.
//!
//! No business logic, no duplicated models. Pure plumbing.
//!
//! # Feature Gate
//!
//! Requires the `cryptogram` feature (implies `std`).

use crate::{
    error::SigningError,
    types::{Signature, WalletType},
};

// Aliases for brevity — the meta-crate provides all three via a single dependency.
use morpheum_crypto::crypto as cc;
use morpheum_crypto::standards as ms;
use morpheum_crypto::types as mst;

// ==================== RE-EXPORTS (Ergonomic Access) ====================

/// Cryptographic primitives — signing and verification functions.
pub mod crypto {
    use super::cc;

    pub use cc::{
        ecdsa_sign, ecdsa_verify,
        ed25519_sign, ed25519_verify, ed25519_public_key,
        schnorr_sign, schnorr_verify, schnorr_x_only_pubkey,
        eip191_sign, eip191_verify, eip191_digest,
        generate_single_from_digest, verify_single_from_digest, verify_multi_from_digest,
        CryptoError,
    };

    pub use cc::{
        bls_sign_digest, bls_verify_digest, bls_aggregate_signatures,
        bls_aggregate_public_keys, bls_verify_aggregated,
    };
}

/// Signature algorithm and chain type identifiers.
pub mod standards_types {
    use super::mst;

    pub use mst::{
        SigType, ChainType, Domain,
        Eip712Tx, Payload,
        Signature as Eip712Signature,
        Witness,
        Address as ChainAddress,
        AuthMethod,
    };
}

/// Agent delegation management (in-memory, chain-aware).
pub mod delegation {
    use super::ms;

    pub use ms::auth::{
        AgentDelegationManager, DelegationInfo, AuthError,
    };
}

/// EIP-712 transaction signing and wallet construction.
pub mod eip712 {
    use super::ms;

    pub use ms::tx::{
        TxSigner as Eip712TxSigner,
        TxBuilder as Eip712TxBuilder,
        Wallet as CryptogramWallet,
        SignatureResult as Eip712SignatureResult,
        hex_signature_to_base64,
    };
}

/// Address validation for all supported chain types.
pub mod validation {
    use super::ms;

    pub use ms::validation::{
        validate_address_for_chain,
        validate_ethereum_address_lenient,
        validate_morpheum_address,
        validate_sui_address,
        validate_ton_address,
        validate_tron_address,
        validate_polkadot_address,
        validate_ada_address,
        ValidationError,
    };
}

/// Nonce management (time-bound high-water-mark).
pub mod nonce_mgmt {
    use super::ms;

    pub use ms::auth::{
        NonceManager, NonceInfo, NonceQueryTag,
    };
}

// ==================== TYPE CONVERSIONS ====================

/// Maps a signing SDK [`WalletType`] to the canonical cryptogram [`SigType`].
///
/// This is the primary bridge between the signing SDK's wallet abstraction
/// and cryptogram's algorithm-level dispatch.
#[must_use]
pub const fn wallet_type_to_sig_type(wt: WalletType) -> mst::SigType {
    use mst::SigType;
    match wt {
        WalletType::Native => SigType::Ed25519,
        WalletType::Evm => SigType::EcdsaLegacyEthereum,
        WalletType::Solana => SigType::Ed25519,
        WalletType::Bitcoin => SigType::SchnorrTaproot,
        WalletType::Agent => SigType::Ed25519,
        WalletType::Hardware => SigType::EcdsaLegacyEthereum,
    }
}

/// Maps a cryptogram [`SigType`] to the signing SDK's [`WalletType`].
///
/// Returns `None` for signature types that don't have a direct wallet mapping
/// (e.g. chain-specific legacy variants, hybrid post-quantum).
#[must_use]
pub fn sig_type_to_wallet_type(st: mst::SigType) -> Option<WalletType> {
    use mst::SigType;
    match st {
        SigType::Ed25519 => Some(WalletType::Native),
        SigType::EcdsaLegacyEthereum
        | SigType::EcdsaLegacy
        | SigType::Keccak256
        | SigType::EcdsaLegacyBitcoin
        | SigType::EcdsaSegwit
        | SigType::EcdsaSegwitLike
        | SigType::EcdsaNestedSegwit
        | SigType::EcdsaLegacyTron
        | SigType::EcdsaLegacySui => Some(WalletType::Evm),
        SigType::SchnorrTaproot | SigType::SchnorrAggregate => Some(WalletType::Bitcoin),
        SigType::Ed25519LegacySui | SigType::Ed25519LegacyTon | SigType::Ed25519LegacyAda => {
            Some(WalletType::Solana)
        }
        _ => None,
    }
}

// ==================== SIGNING DELEGATION ====================

/// Result from cryptogram's universal signing engine.
#[derive(Debug, Clone)]
pub struct CryptogramSignResult {
    /// Hex-encoded signature (with `0x` prefix).
    pub signature_hex: String,
    /// Hex-encoded signer identifier (address or public key, with `0x` prefix).
    pub signer_hex: String,
}

/// Signs a 32-byte digest using cryptogram's universal signing engine.
///
/// Dispatches to the correct algorithm (ECDSA, Ed25519, Schnorr, etc.)
/// based on the provided [`SigType`](morpheum_crypto::types::SigType).
///
/// # Parameters
///
/// - `digest`: The 32-byte message digest to sign.
/// - `sig_type`: Algorithm selector.
/// - `secret_key`: 32-byte secret key.
/// - `signer_address`: Raw signer address bytes (≥20 for ECDSA, ignored for Ed25519/Schnorr).
///
/// # Errors
///
/// Returns [`SigningError`] if the underlying crypto operation fails.
pub fn sign_digest(
    digest: &[u8; 32],
    sig_type: mst::SigType,
    secret_key: &[u8; 32],
    signer_address: &[u8],
) -> Result<CryptogramSignResult, SigningError> {
    let (signature_hex, signer_hex) =
        cc::generate_single_from_digest(digest, sig_type, secret_key, signer_address)
            .map_err(SigningError::from)?;

    Ok(CryptogramSignResult {
        signature_hex,
        signer_hex,
    })
}

/// Verifies one or more signatures against a digest using cryptogram's universal verifier.
///
/// Requires at least `threshold` valid signatures to return `true`.
///
/// # Errors
///
/// Returns [`SigningError`] if the signature type is unsupported.
pub fn verify_digest(
    digest: &[u8; 32],
    signatures: &[mst::eip712_tx::Signature],
    sig_type: mst::SigType,
    threshold: u32,
) -> Result<bool, SigningError> {
    cc::verify_single_from_digest(digest, signatures, sig_type, threshold)
        .map_err(SigningError::from)
}

// ==================== SIGNATURE CONVERSION ====================

/// Converts a hex-encoded signature string to the signing SDK's [`Signature`] type.
///
/// The `sig_type` determines which [`Signature`] variant is returned.
/// Supports 64-byte signatures (Ed25519, Schnorr, ECDSA compact).
///
/// # Errors
///
/// Returns [`SigningError`] if the hex is malformed or the decoded length is not 64 bytes.
pub fn hex_sig_to_signature(
    sig_hex: &str,
    sig_type: mst::SigType,
) -> Result<Signature, SigningError> {
    use mst::SigType;

    let hex_str = sig_hex.strip_prefix("0x").unwrap_or(sig_hex);
    let bytes = hex::decode(hex_str)
        .map_err(|e| SigningError::signing(format!("invalid hex signature: {e}")))?;

    let arr: [u8; 64] = bytes
        .try_into()
        .map_err(|_| SigningError::signing("signature must be 64 bytes for SDK conversion"))?;

    match sig_type {
        SigType::Ed25519
        | SigType::Ed25519LegacyAda
        | SigType::Ed25519LegacySui
        | SigType::Ed25519LegacyTon => Ok(Signature::Ed25519(arr)),
        SigType::SchnorrTaproot | SigType::SchnorrAggregate => Ok(Signature::Schnorr(arr)),
        _ => Ok(Signature::Secp256k1(arr)),
    }
}

// ==================== ADDRESS VALIDATION ====================

/// Validates an address string for a given chain type using cryptogram's validators.
///
/// Delegates to cryptogram's comprehensive chain-type-aware validation
/// which covers Ethereum, Bitcoin (all variants), Solana, Cardano, Sui,
/// TON, Tron, Polkadot, and Morpheum addresses.
///
/// # Errors
///
/// Returns [`SigningError::AddressMapping`] if the address is invalid.
pub fn validate_chain_address(
    address: &str,
    chain_type: mst::ChainType,
) -> Result<(), SigningError> {
    ms::validation::validate_address_for_chain(address, chain_type).map_err(|e| {
        SigningError::AddressMapping {
            address: address.to_string(),
            reason: e.to_string(),
        }
    })
}

// ==================== ERROR CONVERSION ====================

impl From<cc::CryptoError> for SigningError {
    fn from(e: cc::CryptoError) -> Self {
        use crate::error::CryptoError;
        use cc::CryptoError as CE;

        match e {
            CE::InvalidSecretKey => SigningError::invalid_key("invalid secret key"),
            CE::InvalidDigestLen(len) => {
                SigningError::signing(format!("invalid digest length: expected 32, got {len}"))
            }
            CE::InvalidSignatureLen(len) => SigningError::Crypto(CryptoError::Ed25519(format!(
                "invalid signature length: expected 65, got {len}"
            ))),
            CE::InvalidSignatureEncoding(msg) => {
                SigningError::Crypto(CryptoError::Secp256k1(msg))
            }
            CE::RecoveryFailed => {
                SigningError::Crypto(CryptoError::SignatureVerificationFailed)
            }
            CE::UnsupportedSigType(s) => {
                SigningError::signing(format!("unsupported signature type: {s}"))
            }
            CE::EcdsaMldsa44(msg) => SigningError::signing(format!("ECDSA+ML-DSA-44: {msg}")),
            CE::Bls(msg) => SigningError::signing(format!("BLS: {msg}")),
            CE::Other(e) => SigningError::signing(e.to_string()),
        }
    }
}

impl From<ms::auth::AuthError> for SigningError {
    fn from(e: ms::auth::AuthError) -> Self {
        SigningError::custom(e.to_string())
    }
}

impl From<ms::tx::TxError> for SigningError {
    fn from(e: ms::tx::TxError) -> Self {
        SigningError::signing(e.to_string())
    }
}
