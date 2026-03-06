//! Transaction verification for chain-side authentication.
//!
//! This module provides **pure cryptographic verification** of signed Morpheum
//! transactions. It is the canonical entry point for the `auth` module's
//! `TxAuthHotPath`, handling signature validation, `TradingKeyClaim` extraction,
//! and public-key-to-`AccountId` mapping.
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
//! - **DRY**: Delegates to `morpheum_primitives::crypto` for Ed25519, Secp256k1,
//!   and EIP-191 ecrecover — single source of truth for low-level curve crypto.
//!   Only `verify_eip191_personal` (full-key EIP-191, SDK/agent path) is local.

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

    // ── Agent context (extracted from primary signer's SignerInfo) ──

    /// Agent DID string (e.g. `"did:agent:abc123…"`).
    ///
    /// Used by the auth hotpath for identity lookup and shard-affinity routing.
    /// `None` for regular (non-agent) transactions — zero overhead.
    pub agent_did: Option<String>,

    /// Raw Verifiable Presentation bytes.
    ///
    /// Decoded and verified by the VC hotpath for delegation claims
    /// (max daily USD, allowed pairs, etc.).
    pub verifiable_presentation: Option<Vec<u8>>,

    /// Delegated trading key address.
    ///
    /// Checked against pre-approved keys (via `MsgApproveTradingKey`)
    /// in the auth keeper. Enables nonce sub-range isolation for parallelism.
    pub trading_key_address: Option<String>,
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

    // ── Extract agent context from primary signer ──
    let primary_si = &signer_infos[0];

    Ok(VerifiedTx {
        account_ids,
        trading_key_claim,
        wallet_type: primary_wallet_type,
        sign_mode: primary_sign_mode,
        body: body.clone(),
        auth_info: auth_info.clone(),
        nonce: tx.nonce,
        agent_did: primary_si.agent_did.clone(),
        verifiable_presentation: primary_si.verifiable_presentation.clone(),
        trading_key_address: primary_si.trading_key_address.clone(),
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
/// | PublicKey variant | SignMode(s)                                | Verifier            |
/// |-------------------|--------------------------------------------|---------------------|
/// | Ed25519 / Agent   | Ed25519, GaslessEd25519                    | ed25519-dalek       |
/// | Ed25519 / Agent   | SolanaOffchain                             | ed25519 hex-encoded |
/// | Secp256k1         | Secp256k1, EcdsaLegacy, Keccak256          | k256 ECDSA          |
/// | Secp256k1         | Eip191Personal                             | k256 + sha3         |
/// | EvmAddress        | Eip191Personal                             | k256 ecrecover      |
/// | Schnorr           | SchnorrAggregate                           | (unsupported)       |
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

        // ── Secp256k1 raw ECDSA (standard, pre-hashed) ──
        (PublicKey::Secp256k1(key_bytes),
         SignMode::Secp256k1 | SignMode::EcdsaLegacy | SignMode::Keccak256) => {
            verify_secp256k1(key_bytes, sign_doc_bytes, sig_bytes)
        }

        // ── EIP-191 personal_sign with full compressed public key ──
        //
        // When the client provides the full 33-byte compressed secp256k1 key
        // (e.g., from SDK or agent context where the key is known), we can
        // directly verify the ECDSA signature against the EIP-191 hash.
        (PublicKey::Secp256k1(key_bytes),
         SignMode::Eip191Personal) => {
            verify_eip191_personal(key_bytes, sign_doc_bytes, sig_bytes)
        }

        // ── EIP-191 personal_sign with EVM address only (ecrecover) ──
        //
        // MetaMask / EVM wallet flow: the client only provides its 20-byte
        // Ethereum address. The verifier recovers the public key from the
        // 65-byte signature (r‖s‖v), derives the EVM address from the
        // recovered key, and compares it to the expected address.
        (PublicKey::EvmAddress(expected_addr),
         SignMode::Eip191Personal) => {
            verify_eip191_ecrecover(expected_addr, sign_doc_bytes, sig_bytes)
        }

        // ── Solana off-chain message (hex-encoded SignDoc via Phantom) ──
        //
        // Phantom's `signMessage` converts bytes through UTF-8 internally,
        // which corrupts non-ASCII binary data. The client hex-encodes the
        // SignDoc bytes before passing to the wallet, producing an ASCII
        // string that survives the UTF-8 round-trip losslessly.
        // We reproduce the same hex encoding here before Ed25519 verification.
        (PublicKey::Ed25519(key_bytes) | PublicKey::Agent(key_bytes),
         SignMode::SolanaOffchain) => {
            verify_ed25519_hex_encoded(key_bytes, sign_doc_bytes, sig_bytes)
        }

        // ── Unsupported combinations ──
        _ => Err(SigningError::Crypto(CryptoError::UnsupportedCurve)),
    }
}

/// Ed25519 signature verification — delegates to `morpheum_primitives::crypto`.
///
/// Single source of truth: `verify_ed25519_bytes` in primitives performs
/// strict Ed25519 verification via `ed25519-dalek`. This wrapper maps the
/// primitives error to `SigningError`.
fn verify_ed25519(
    key_bytes: &[u8; 32],
    message: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    morpheum_primitives::crypto::verify_ed25519_bytes(key_bytes, message, sig_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Ed25519(
            alloc::format!("{e}")
        )))
}

/// Secp256k1 ECDSA signature verification — delegates to `morpheum_primitives::crypto`.
///
/// Single source of truth: `verify_secp256k1_bytes` in primitives uses
/// `k256::ecdsa::Verifier::verify` which internally SHA-256 hashes the
/// message before ECDSA verification (standard convention). The signing
/// side must use `SigningKey::sign(message)` which applies the same hash.
fn verify_secp256k1(
    key_bytes: &[u8; 33],
    message: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    morpheum_primitives::crypto::verify_secp256k1_bytes(key_bytes, message, sig_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("{e}")
        )))
}

/// EIP-191 `personal_sign` verification using `k256` + `sha3` (Keccak-256).
///
/// MetaMask (and other EVM wallets) sign via `personal_sign`, which:
/// 1. Prepends the EIP-191 prefix: `"\x19Ethereum Signed Message:\n" + decimal_len`
/// 2. Concatenates the raw message bytes
/// 3. Computes `keccak256` of the prefixed message
/// 4. Signs the 32-byte hash with ECDSA (secp256k1)
/// 5. Returns a 65-byte signature: `r(32) || s(32) || v(1)`
///
/// This function reconstructs the same keccak256 hash from `sign_doc_bytes`
/// and verifies the ECDSA signature against the declared compressed public key.
/// The recovery byte `v` is stripped if a 65-byte signature is provided.
fn verify_eip191_personal(
    key_bytes: &[u8; 33],
    sign_doc_bytes: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    use k256::ecdsa::{
        signature::hazmat::PrehashVerifier,
        Signature as SecpSig, VerifyingKey,
    };
    use sha3::{Digest, Keccak256};

    // 1. Reconstruct the EIP-191 personal_sign hash.
    //    This matches what MetaMask computes internally:
    //    keccak256("\x19Ethereum Signed Message:\n" + decimal_len(msg) + msg)
    let prefix = alloc::format!("\x19Ethereum Signed Message:\n{}", sign_doc_bytes.len());
    let hash: [u8; 32] = {
        let mut keccak = Keccak256::new();
        keccak.update(prefix.as_bytes());
        keccak.update(sign_doc_bytes);
        keccak.finalize().into()
    };

    // 2. Parse the compressed secp256k1 public key (33 bytes).
    let verifying_key = VerifyingKey::from_sec1_bytes(key_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("invalid secp256k1 public key: {e}")
        )))?;

    // 3. Extract the 64-byte ECDSA signature (r || s).
    //    MetaMask returns 65 bytes (r:32 + s:32 + v:1); strip the recovery
    //    byte `v` for standard ECDSA verification.
    let sig_data = match sig_bytes.len() {
        65 => &sig_bytes[..64],
        64 => sig_bytes,
        len => return Err(SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("EIP-191 signature must be 64 or 65 bytes, got {len}")
        ))),
    };

    let signature = SecpSig::from_slice(sig_data)
        .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("invalid EIP-191 secp256k1 signature: {e}")
        )))?;

    // 4. Verify the signature against the EIP-191 keccak256 hash.
    verifying_key.verify_prehash(&hash, &signature)
        .map_err(|_| SigningError::Crypto(CryptoError::SignatureVerificationFailed))
}

/// EIP-191 `personal_sign` verification via **ecrecover** — delegates to
/// `morpheum_primitives::crypto::eip191_ecrecover_verify`.
///
/// Single source of truth: primitives handles the full EIP-191 envelope
/// reconstruction, public key recovery, and address comparison. This wrapper
/// maps the primitives error to `SigningError`.
fn verify_eip191_ecrecover(
    expected_addr: &[u8; 20],
    sign_doc_bytes: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    morpheum_primitives::crypto::eip191_ecrecover_verify(expected_addr, sign_doc_bytes, sig_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Secp256k1(
            alloc::format!("{e}")
        )))
}

/// Ed25519 verification for Solana off-chain messages (hex-encoded SignDoc).
///
/// Phantom's `signMessage` API internally converts the message bytes through
/// `Buffer.toString("utf-8")`, which is lossy for non-ASCII binary data
/// (invalid UTF-8 sequences are replaced with U+FFFD). To avoid this
/// corruption, the client hex-encodes the `SignDoc` bytes before passing
/// them to the wallet. The wallet signs the UTF-8-safe hex string bytes.
///
/// This function reproduces the same hex encoding on the chain side:
/// it hex-encodes `sign_doc_bytes` and verifies the Ed25519 signature
/// against the resulting ASCII byte string.
fn verify_ed25519_hex_encoded(
    key_bytes: &[u8; 32],
    sign_doc_bytes: &[u8],
    sig_bytes: &[u8],
) -> Result<(), SigningError> {
    let hex_encoded = hex::encode(sign_doc_bytes);
    morpheum_primitives::crypto::verify_ed25519_bytes(key_bytes, hex_encoded.as_bytes(), sig_bytes)
        .map_err(|e| SigningError::Crypto(CryptoError::Ed25519(
            alloc::format!("{e}")
        )))
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
