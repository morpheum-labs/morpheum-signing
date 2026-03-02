//! Transaction verification for chain-side authentication.
//!
//! This module provides **pure cryptographic verification** of signed Morpheum
//! transactions. It is the single source of truth for signature validation,
//! `TradingKeyClaim` extraction, and public-key-to-`AccountId` mapping.
//!
//! # Feature Gate
//!
//! Requires the `full-crypto` feature (enables `ed25519-dalek` and `k256`).
//!
//! # Design
//!
//! - **Pure function**: No state, no I/O — only cryptographic verification.
//! - **Multi-curve**: Dispatches Ed25519, Secp256k1 based on `SignerInfo.mode_info`.
//! - **Constant-time**: All signature verification uses constant-time operations.
//! - **DRY**: Mormcore's `auth` module calls this instead of reimplementing crypto.

use alloc::string::String;
use alloc::vec::Vec;

use prost::Message;

use crate::{
    claim::TradingKeyClaim,
    error::{CryptoError, SigningError},
    proto::tx::v1::{self as tx, AuthInfo, SignDoc, SignMode, SignerInfo, TxBody},
    types::{AccountId, PublicKey, SignedTx, WalletType},
};

// ============================================================================
// VERIFIED TX (Output of Pure Crypto Verification)
// ============================================================================

/// Result of successful cryptographic transaction verification.
///
/// Contains all information extracted from the signed transaction that the
/// auth module needs for stateful checks (nonce, mana, identity, VC).
/// This struct is the **bridge** between pure crypto (signing-core) and
/// stateful authentication (mormcore auth module).
#[derive(Debug, Clone)]
pub struct VerifiedTx {
    /// Canonical account IDs derived from each signer's public key.
    ///
    /// Position-matched to `auth_info.signer_infos`. The first entry is the
    /// primary account (fee payer / transaction initiator).
    pub account_ids: Vec<AccountId>,

    /// Extracted and structurally verified `TradingKeyClaim` (agent flow only).
    ///
    /// Present when a signer's `signing_options` contains a claim with
    /// `algo_hint == "trading_key_claim"`. The claim's issuer has been
    /// verified to match the signer's public key, and expiry has been checked.
    pub trading_key_claim: Option<TradingKeyClaim>,

    /// Primary wallet type (inferred from first signer's public key curve).
    pub wallet_type: WalletType,

    /// Primary sign mode (from first signer's `mode_info`).
    pub sign_mode: tx::SignMode,

    /// Decoded transaction body (messages, memo, timeout).
    pub body: TxBody,

    /// Decoded auth info (signer infos for downstream use).
    pub auth_info: AuthInfo,

    /// Transaction nonce (from `Tx.nonce` if present).
    pub nonce: Option<tx::Nonce>,
}

// ============================================================================
// MAIN VERIFICATION FUNCTION
// ============================================================================

/// Performs pure cryptographic verification of a [`SignedTx`].
///
/// This is the **canonical entry point** for chain-side transaction authentication.
/// It verifies all signatures, extracts `TradingKeyClaim`s, maps public keys to
/// `AccountId`s, and returns a [`VerifiedTx`] ready for stateful auth checks.
///
/// # Parameters
///
/// - `signed_tx`: The decoded transaction (from [`SignedTx::decode`]).
/// - `chain_id`: The chain identifier from node configuration.
/// - `account_number`: The on-chain account number (for `SignDoc` reconstruction).
///   Use `0` if the chain does not enforce account-number binding.
/// - `now`: Current Unix timestamp in seconds (for `TradingKeyClaim` expiry).
///
/// # Errors
///
/// Returns [`SigningError`] if:
/// - The transaction is missing required fields (`body`, `auth_info`).
/// - Signer count mismatches signature count.
/// - Any signature fails cryptographic verification.
/// - A `TradingKeyClaim` is malformed, expired, or has issuer mismatch.
pub fn verify_signed_tx(
    signed_tx: &SignedTx,
    chain_id: &str,
    account_number: u64,
    now: u64,
) -> Result<VerifiedTx, SigningError> {
    let tx = &signed_tx.tx;

    // ── Extract and validate required transaction components ──
    let body = tx.body.as_ref()
        .ok_or_else(|| SigningError::signing("transaction missing body"))?;
    let auth_info = tx.auth_info.as_ref()
        .ok_or_else(|| SigningError::signing("transaction missing auth_info"))?;

    let signer_infos = &auth_info.signer_infos;
    let signatures = &tx.signatures;

    if signer_infos.len() != signatures.len() {
        return Err(SigningError::signing(alloc::format!(
            "signer_infos count ({}) != signatures count ({})",
            signer_infos.len(),
            signatures.len(),
        )));
    }

    if signer_infos.is_empty() {
        return Err(SigningError::signing("transaction has no signers"));
    }

    // ── Reconstruct the canonical SignDoc bytes ──
    //
    // Uses body_bytes and auth_info_bytes from TxRaw if available (exact byte
    // preservation from the wire), otherwise re-encodes from decoded fields.
    let (body_bytes, auth_info_bytes) = match &signed_tx.tx_raw {
        Some(tx_raw) => (tx_raw.body_bytes.clone(), tx_raw.auth_info_bytes.clone()),
        None => (body.encode_to_vec(), auth_info.encode_to_vec()),
    };

    let sign_doc = SignDoc {
        body_bytes,
        auth_info_bytes,
        chain_id: chain_id.into(),
        account_number,
    };
    let sign_doc_bytes = sign_doc.encode_to_vec();

    // ── Verify each signer ──
    let mut account_ids = Vec::with_capacity(signer_infos.len());
    let mut trading_key_claim: Option<TradingKeyClaim> = None;
    let mut primary_sign_mode = SignMode::Ed25519;
    let mut primary_wallet_type = WalletType::Native;

    for (i, si) in signer_infos.iter().enumerate() {
        // 1. Parse public key from proto Any
        let pk_any = si.public_key.as_ref()
            .ok_or_else(|| SigningError::signing(alloc::format!(
                "signer_info[{i}] missing public_key"
            )))?;
        let pubkey = PublicKey::from_proto_any(pk_any)?;

        // 2. Determine sign mode from mode_info
        let mode = extract_sign_mode(si);

        // 3. Cryptographic signature verification
        verify_signature(&pubkey, &sign_doc_bytes, &signatures[i], mode)?;

        // 4. Map public key to canonical AccountId
        account_ids.push(pubkey.to_account_id());

        // 5. Extract TradingKeyClaim if present (agent flow)
        if let Some(opts) = &si.signing_options {
            if let Some(claim) = TradingKeyClaim::decode_from_signing_options(opts)? {
                // Structural + issuer verification (expiry + issuer-pubkey match)
                #[cfg(feature = "claim-verification")]
                claim.verify(now, &pubkey)?;

                #[cfg(not(feature = "claim-verification"))]
                claim.validate(now)?;

                trading_key_claim = Some(claim);
            }
        }

        // Record primary signer metadata
        if i == 0 {
            primary_sign_mode = mode;
            primary_wallet_type = pubkey.infer_wallet_type();
        }
    }

    Ok(VerifiedTx {
        account_ids,
        trading_key_claim,
        wallet_type: primary_wallet_type,
        sign_mode: primary_sign_mode,
        body: body.clone(),
        auth_info: auth_info.clone(),
        nonce: tx.nonce.clone(),
    })
}

// ============================================================================
// SIGNATURE VERIFICATION (Per-Curve Dispatch)
// ============================================================================

/// Verifies a cryptographic signature against the canonical `SignDoc` bytes.
///
/// Dispatches to the appropriate curve verifier based on the signer's public
/// key type and the declared `SignMode`. All verification is constant-time
/// with respect to secret material.
///
/// # Supported Curves
///
/// | PublicKey variant | SignMode(s)                                | Library        |
/// |-------------------|--------------------------------------------|----------------|
/// | Ed25519 / Agent   | Ed25519, GaslessEd25519                    | ed25519-dalek  |
/// | Secp256k1         | Secp256k1, EcdsaLegacy, Keccak256          | k256           |
/// | Schnorr           | SchnorrAggregate                           | (unsupported)  |
fn verify_signature(
    pubkey: &PublicKey,
    sign_doc_bytes: &[u8],
    sig_bytes: &[u8],
    mode: SignMode,
) -> Result<(), SigningError> {
    match (pubkey, mode) {
        // ── Ed25519 (Native, Solana, Agent) ──
        (PublicKey::Ed25519(key_bytes) | PublicKey::Agent(key_bytes),
         SignMode::Ed25519 | SignMode::GaslessEd25519) => {
            verify_ed25519(key_bytes, sign_doc_bytes, sig_bytes)
        }

        // ── Secp256k1 (EVM / MetaMask) ──
        (PublicKey::Secp256k1(key_bytes),
         SignMode::Secp256k1 | SignMode::EcdsaLegacy | SignMode::Keccak256) => {
            verify_secp256k1(key_bytes, sign_doc_bytes, sig_bytes)
        }

        // ── Unsupported combinations ──
        _ => Err(SigningError::Crypto(CryptoError::UnsupportedCurve)),
    }
}

/// Ed25519 signature verification using `ed25519-dalek` (strict mode).
fn verify_ed25519(
    key_bytes: &[u8; 32],
    message: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    use ed25519_dalek::{Signature as DalekSig, VerifyingKey};

    let verifying_key = VerifyingKey::from_bytes(key_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Ed25519(
            alloc::format!("invalid ed25519 public key: {e}")
        )))?;

    let sig_arr: [u8; 64] = sig_bytes.try_into()
        .map_err(|_| SigningError::Crypto(CryptoError::Ed25519(
            String::from("signature must be 64 bytes")
        )))?;
    let signature = DalekSig::from_bytes(&sig_arr);

    verifying_key.verify_strict(message, &signature)
        .map_err(|_| SigningError::Crypto(CryptoError::SignatureVerificationFailed))
}

/// Secp256k1 ECDSA signature verification using `k256`.
///
/// Uses `verify_prehash` to match the signing side's `sign_prehash` convention
/// (raw `SignDoc` bytes are treated as the pre-hashed message).
fn verify_secp256k1(
    key_bytes: &[u8; 33],
    message: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    use k256::ecdsa::{
        signature::hazmat::PrehashVerifier,
        Signature as SecpSig, VerifyingKey,
    };

    let verifying_key = VerifyingKey::from_sec1_bytes(key_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("invalid secp256k1 public key: {e}")
        )))?;

    let signature = SecpSig::from_slice(sig_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("invalid secp256k1 signature: {e}")
        )))?;

    verifying_key.verify_prehash(message, &signature)
        .map_err(|_| SigningError::Crypto(CryptoError::SignatureVerificationFailed))
}

// ============================================================================
// HELPERS
// ============================================================================

/// Extracts the [`SignMode`] from a `SignerInfo`'s nested `mode_info`.
///
/// Falls back to `SignMode::Ed25519` if the mode cannot be determined
/// (e.g., missing `mode_info` or unrecognized enum value).
fn extract_sign_mode(si: &SignerInfo) -> SignMode {
    si.mode_info.as_ref()
        .and_then(|mi| mi.sum.as_ref())
        .and_then(|sum| match sum {
            tx::mode_info::Sum::Single(single) => {
                SignMode::try_from(single.mode).ok()
            }
            _ => None,
        })
        .unwrap_or(SignMode::Ed25519)
}
