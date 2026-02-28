//! VC and TradingKeyClaim helpers for agent delegation in Morpheum.
//!
//! Provides immutable value objects and a fluent builder for creating
//! TradingKeyClaims that are embedded in `Tx.AuthInfo` or `SignerInfo`
//! for fine-grained delegation with nonce sub-range isolation.
//!
//! This module is deliberately lightweight and focused — it only constructs
//! and validates claims. Actual signing happens in the `Signer` trait.

use crate::{
    error::SigningError,
    mapper::AddressMapper,
    types::{AccountId, Address, Signature},
};

use prost_types::Any;

/// TradingKeyClaim — immutable value object for agent delegation.
///
/// This is the exact claim verified by `auth::NonceHotPath` and `clob` hot-paths.
/// It enables secondary keys (TradingKeys) to sign with isolated nonce sub-ranges
/// while respecting owner-defined limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradingKeyClaim {
    /// Owner who issued the claim (must match the main account owner)
    pub issuer: AccountId,
    /// Agent/subject that can use this claim
    pub subject: AccountId,
    /// Permission bitflags (TRADE, EVALUATE, etc.)
    pub permissions: u64,
    /// Daily USD spending limit
    pub max_daily_usd: u64,
    /// Expiry timestamp (Unix seconds)
    pub expiry_timestamp: u64,
    /// Start of the nonce sub-range (inclusive)
    pub nonce_sub_range_start: u32,
    /// End of the nonce sub-range (exclusive)
    pub nonce_sub_range_end: u32,
    /// Signature by the issuer (verified in auth hotpath)
    pub signature: Signature,
}

impl TradingKeyClaim {
    /// Validates the claim (expiry, range, etc.).
    pub fn validate(&self, current_timestamp: u64) -> Result<(), SigningError> {
        if self.expiry_timestamp <= current_timestamp {
            return Err(SigningError::invalid_claim("claim has expired"));
        }
        if self.nonce_sub_range_start >= self.nonce_sub_range_end {
            return Err(SigningError::invalid_claim("invalid nonce sub-range"));
        }
        if self.signature.0.is_empty() {
            return Err(SigningError::invalid_claim("missing signature"));
        }
        Ok(())
    }

    /// Returns the size of the nonce sub-range for parallelism.
    pub fn sub_range_size(&self) -> u32 {
        self.nonce_sub_range_end.saturating_sub(self.nonce_sub_range_start)
    }

    /// Packs the claim into a protobuf `Any` for embedding in `Tx.AuthInfo` or `SignerInfo`.
    pub fn into_any(self) -> Any {
        // In a real implementation this would serialize to a dedicated protobuf message.
        // For now we use a placeholder — the actual proto message would be in primitives.
        Any {
            type_url: "type.googleapis.com/morpheum.vc.v1.TradingKeyClaim".to_string(),
            value: vec![], // Replace with real serialization in full primitives integration
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issuer(mut self, issuer: AccountId) -> Self {
        self.issuer = Some(issuer);
        self
    }

    pub fn subject(mut self, subject: AccountId) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn permissions(mut self, permissions: u64) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn max_daily_usd(mut self, amount: u64) -> Self {
        self.max_daily_usd = amount;
        self
    }

    pub fn expiry(mut self, timestamp: u64) -> Self {
        self.expiry_timestamp = Some(timestamp);
        self
    }

    pub fn nonce_sub_range(mut self, start: u32, end: u32) -> Self {
        self.nonce_sub_range_start = start;
        self.nonce_sub_range_end = end;
        self
    }

    pub fn signature(mut self, sig: Signature) -> Self {
        self.signature = Some(sig);
        self
    }

    /// Builds and validates the claim.
    ///
    /// Requires an `AddressMapper` only if you pass `Address` instead of `AccountId`.
    pub fn build(self) -> Result<TradingKeyClaim, SigningError> {
        let issuer = self.issuer.ok_or_else(|| SigningError::invalid_claim("issuer is required"))?;
        let subject = self.subject.ok_or_else(|| SigningError::invalid_claim("subject is required"))?;
        let expiry = self.expiry_timestamp.ok_or_else(|| SigningError::invalid_claim("expiry is required"))?;
        let signature = self.signature.ok_or_else(|| SigningError::invalid_claim("signature is required"))?;

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

        claim.validate(chrono::Utc::now().timestamp() as u64)?; // Replace with proper time source in production

        Ok(claim)
    }
}