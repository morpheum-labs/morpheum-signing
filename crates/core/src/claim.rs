//! VC and `TradingKeyClaim` helpers for agent delegation in Morpheum.
//!
//! Provides immutable value objects and a fluent builder for creating
//! `TradingKeyClaims` that are embedded in `Tx.AuthInfo` or `SignerInfo`
//! for fine-grained delegation with nonce sub-range isolation.
//!
//! This module is deliberately lightweight and focused — it only constructs,
//! validates, serializes, and verifies claims. Actual signing happens in the `Signer` trait.

use alloc::vec::Vec;

use prost::Message;
use crate::proto::Any;
use sha2::{Digest, Sha256};

use crate::{
    error::SigningError,
    types::{AccountId, Signature},
};

/// The canonical protobuf type URL for [`TradingKeyClaim`].
///
/// Matches the chain-side `vc.v1.TradingKeyClaim` proto message and is
/// used as the `type_url` in proto `Any` encoding.
pub const TRADING_KEY_CLAIM_TYPE_URL: &str =
    "type.googleapis.com/morpheum.signing.v1.TradingKeyClaim";

/// Internal protobuf-compatible encoding for [`TradingKeyClaim`].
///
/// Mirrors what a `.proto` file would generate, ensuring deterministic,
/// forward-compatible binary encoding via prost. Not exposed publicly —
/// callers use [`TradingKeyClaim::encode_to_vec`] and [`TradingKeyClaim::to_proto_any`].
#[derive(prost::Message)]
struct TradingKeyClaimProto {
    #[prost(bytes = "vec", tag = "1")]
    pub issuer: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub subject: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub permissions: u64,
    #[prost(uint64, tag = "4")]
    pub max_daily_usd: u64,
    #[prost(uint64, tag = "5")]
    pub expiry_timestamp: u64,
    #[prost(uint32, tag = "6")]
    pub nonce_sub_range_start: u32,
    #[prost(uint32, tag = "7")]
    pub nonce_sub_range_end: u32,
    #[prost(bytes = "vec", tag = "8")]
    pub signature: Vec<u8>,
}

/// `TradingKeyClaim` — immutable value object for agent delegation.
///
/// This is the exact claim verified by `auth::NonceHotPath` and `clob` hot-paths.
/// It enables secondary keys (`TradingKeys`) to sign with isolated nonce sub-ranges
/// while respecting owner-defined limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradingKeyClaim {
    /// Owner who issued the claim (must match the main account owner).
    pub issuer: AccountId,
    /// Agent/subject that can use this claim.
    pub subject: AccountId,
    /// Permission bitflags (TRADE, EVALUATE, etc.).
    pub permissions: u64,
    /// Daily USD spending limit.
    pub max_daily_usd: u64,
    /// Expiry timestamp (Unix seconds).
    pub expiry_timestamp: u64,
    /// Start of the nonce sub-range (inclusive).
    pub nonce_sub_range_start: u32,
    /// End of the nonce sub-range (exclusive).
    pub nonce_sub_range_end: u32,
    /// Signature by the issuer (verified in auth hotpath).
    pub signature: Signature,
}

impl TradingKeyClaim {
    /// Validates the claim against a given timestamp.
    ///
    /// Checks expiry, nonce sub-range validity, and signature presence.
    ///
    /// # Errors
    ///
    /// Returns `SigningError::InvalidClaim` if the claim is expired, has an invalid
    /// nonce sub-range, or is missing a signature.
    pub fn validate(&self, current_timestamp: u64) -> Result<(), SigningError> {
        if self.expiry_timestamp <= current_timestamp {
            return Err(SigningError::invalid_claim("claim has expired"));
        }
        if self.nonce_sub_range_start >= self.nonce_sub_range_end {
            return Err(SigningError::invalid_claim("invalid nonce sub-range"));
        }
        if self.signature.is_zero() {
            return Err(SigningError::invalid_claim("missing signature"));
        }
        Ok(())
    }

    /// Verifies the claim against the current time and issuer's public key.
    ///
    /// Performs:
    /// 1. **Structural validation**: expiry, nonce sub-range, signature presence.
    /// 2. **Issuer consistency**: the claim's `issuer` [`AccountId`] must match
    ///    the canonical `AccountId` derived from `issuer_pubkey`.
    ///
    /// # Cryptographic Signature Verification
    ///
    /// Full curve-specific signature verification (ed25519, secp256k1, etc.)
    /// requires the native crate's crypto dependencies. This method provides
    /// all non-crypto checks; the chain-side performs authoritative cryptographic
    /// verification via the VC hot-path.
    ///
    /// Use [`claim_digest`](Self::claim_digest) to obtain the signed-over bytes
    /// for external signature verification.
    ///
    /// # Errors
    ///
    /// Returns `SigningError::InvalidClaim` if any validation check fails.
    #[cfg(feature = "claim-verification")]
    pub fn verify(
        &self,
        now_secs: u64,
        issuer_pubkey: &crate::types::PublicKey,
    ) -> Result<(), SigningError> {
        self.validate(now_secs)?;

        let expected_issuer = issuer_pubkey.to_account_id();
        if self.issuer != expected_issuer {
            return Err(SigningError::claim_verification(
                "issuer does not match the provided public key",
            ));
        }

        Ok(())
    }

    /// Returns the size of the nonce sub-range for parallelism.
    #[must_use]
    pub const fn sub_range_size(&self) -> u32 {
        self.nonce_sub_range_end.saturating_sub(self.nonce_sub_range_start)
    }

    /// Computes the SHA-256 digest of the unsigned claim fields.
    ///
    /// This is the canonical digest that the issuer signs when creating the claim.
    /// The `signature` field is excluded (it is the signature *over* this digest).
    ///
    /// The encoding uses prost's deterministic protobuf serialization for
    /// forward compatibility with `.proto`-generated decoders on the chain side.
    #[must_use]
    pub fn claim_digest(&self) -> [u8; 32] {
        let unsigned = TradingKeyClaimProto {
            issuer: self.issuer.0.to_vec(),
            subject: self.subject.0.to_vec(),
            permissions: self.permissions,
            max_daily_usd: self.max_daily_usd,
            expiry_timestamp: self.expiry_timestamp,
            nonce_sub_range_start: self.nonce_sub_range_start,
            nonce_sub_range_end: self.nonce_sub_range_end,
            signature: Vec::new(), // Excluded from digest
        };
        let bytes = unsigned.encode_to_vec();
        let hash = Sha256::digest(&bytes);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        arr
    }

    /// Encodes the claim into deterministic protobuf-compatible bytes.
    ///
    /// Uses prost's encoding for consistency with chain-side proto decoders.
    #[must_use]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        self.to_proto_inner().encode_to_vec()
    }

    /// Packs the claim into a proto `Any` for embedding in `SignerInfo`.
    ///
    /// The `type_url` is [`TRADING_KEY_CLAIM_TYPE_URL`] and the `value` is the
    /// full prost-encoded claim (including signature).
    #[must_use]
    pub fn to_proto_any(&self) -> Any {
        Any {
            type_url: TRADING_KEY_CLAIM_TYPE_URL.into(),
            value: self.encode_to_vec(),
        }
    }

    /// Consumes the claim and packs it into a proto `Any`.
    ///
    /// Equivalent to [`to_proto_any`](Self::to_proto_any) but takes ownership.
    #[must_use]
    pub fn into_any(self) -> Any {
        Any {
            type_url: TRADING_KEY_CLAIM_TYPE_URL.into(),
            value: self.encode_to_vec(),
        }
    }

    /// Converts to the internal prost-derived representation for encoding.
    fn to_proto_inner(&self) -> TradingKeyClaimProto {
        TradingKeyClaimProto {
            issuer: self.issuer.0.to_vec(),
            subject: self.subject.0.to_vec(),
            permissions: self.permissions,
            max_daily_usd: self.max_daily_usd,
            expiry_timestamp: self.expiry_timestamp,
            nonce_sub_range_start: self.nonce_sub_range_start,
            nonce_sub_range_end: self.nonce_sub_range_end,
            signature: self.signature.to_bytes(),
        }
    }
}

/// Fluent builder for `TradingKeyClaim`.
///
/// This is the recommended way to create claims for agents.
#[derive(Debug, Default)]
pub struct VcClaimBuilder {
    issuer: Option<AccountId>,
    subject: Option<AccountId>,
    permissions: u64,
    max_daily_usd: u64,
    expiry_timestamp: Option<u64>,
    nonce_sub_range_start: u32,
    nonce_sub_range_end: u32,
    signature: Option<Signature>,
}

impl VcClaimBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the issuer `AccountId`.
    #[must_use]
    pub fn issuer(mut self, issuer: AccountId) -> Self {
        self.issuer = Some(issuer);
        self
    }

    /// Sets the subject `AccountId`.
    #[must_use]
    pub fn subject(mut self, subject: AccountId) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Sets the permission bitflags.
    #[must_use]
    pub const fn permissions(mut self, permissions: u64) -> Self {
        self.permissions = permissions;
        self
    }

    /// Sets the daily USD spending limit.
    #[must_use]
    pub const fn max_daily_usd(mut self, amount: u64) -> Self {
        self.max_daily_usd = amount;
        self
    }

    /// Sets the expiry timestamp (Unix seconds).
    #[must_use]
    pub const fn expiry(mut self, timestamp: u64) -> Self {
        self.expiry_timestamp = Some(timestamp);
        self
    }

    /// Sets the nonce sub-range [start, end).
    #[must_use]
    pub const fn nonce_sub_range(mut self, start: u32, end: u32) -> Self {
        self.nonce_sub_range_start = start;
        self.nonce_sub_range_end = end;
        self
    }

    /// Sets the issuer's signature over the claim.
    #[must_use]
    pub fn signature(mut self, sig: Signature) -> Self {
        self.signature = Some(sig);
        self
    }

    /// Builds and validates the claim.
    ///
    /// The `current_timestamp` is used for expiry validation (Unix seconds).
    ///
    /// # Errors
    ///
    /// Returns `SigningError::InvalidClaim` if required fields are missing or the
    /// built claim fails validation.
    pub fn build(self, current_timestamp: u64) -> Result<TradingKeyClaim, SigningError> {
        let issuer = self
            .issuer
            .ok_or_else(|| SigningError::invalid_claim("issuer is required"))?;
        let subject = self
            .subject
            .ok_or_else(|| SigningError::invalid_claim("subject is required"))?;
        let expiry = self
            .expiry_timestamp
            .ok_or_else(|| SigningError::invalid_claim("expiry is required"))?;
        let signature = self
            .signature
            .ok_or_else(|| SigningError::invalid_claim("signature is required"))?;

        let claim = TradingKeyClaim {
            issuer,
            subject,
            permissions: self.permissions,
            max_daily_usd: self.max_daily_usd,
            expiry_timestamp: expiry,
            nonce_sub_range_start: self.nonce_sub_range_start,
            nonce_sub_range_end: self.nonce_sub_range_end,
            signature,
        };

        claim.validate(current_timestamp)?;

        Ok(claim)
    }
}
