//! VC and `TradingKeyClaim` helpers for agent delegation in Morpheum.
//!
//! Provides immutable value objects and a fluent builder for creating
//! `TradingKeyClaims` that are embedded in `Tx.AuthInfo` or `SignerInfo`
//! for fine-grained delegation with nonce sub-range isolation.
//!
//! This module is deliberately lightweight and focused — it only constructs,
//! validates, serializes, and verifies claims. Actual signing happens in the `Signer` trait.

use crate::proto::Any;
use prost::Message;
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
        self.nonce_sub_range_end
            .saturating_sub(self.nonce_sub_range_start)
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

    /// Decodes a [`TradingKeyClaim`] from a `SignerInfo.signing_options` field.
    ///
    /// Returns `Ok(None)` if the signing options do not contain a `TradingKeyClaim`
    /// (i.e., `algo_hint` is not `"trading_key_claim"`). Returns `Ok(Some(claim))`
    /// if successfully decoded, or `Err` if the data is malformed.
    ///
    /// # Encoding Convention
    ///
    /// The `TxBuilder` encodes the claim as:
    /// 1. `TradingKeyClaim` → proto bytes → `Any { type_url, value }`
    /// 2. `Any` → proto bytes → stored in `SigningOptions.wasm_seed`
    /// 3. `SigningOptions.algo_hint` = `"trading_key_claim"`
    pub fn decode_from_signing_options(
        opts: &crate::proto::tx::v1::SigningOptions,
    ) -> Result<Option<Self>, SigningError> {
        if opts.algo_hint != "trading_key_claim" {
            return Ok(None);
        }

        let any = Any::decode(opts.wasm_seed.as_slice()).map_err(|e| {
            SigningError::invalid_claim(format!("failed to decode claim Any envelope: {e}"))
        })?;

        if any.type_url != TRADING_KEY_CLAIM_TYPE_URL {
            return Ok(None);
        }

        let proto = TradingKeyClaimProto::decode(any.value.as_slice()).map_err(|e| {
            SigningError::invalid_claim(format!("failed to decode TradingKeyClaim proto: {e}"))
        })?;

        let issuer_bytes: [u8; 32] = proto
            .issuer
            .try_into()
            .map_err(|_| SigningError::invalid_claim("issuer must be 32 bytes"))?;
        let subject_bytes: [u8; 32] = proto
            .subject
            .try_into()
            .map_err(|_| SigningError::invalid_claim("subject must be 32 bytes"))?;

        let sig_bytes: [u8; 64] = proto
            .signature
            .try_into()
            .map_err(|_| SigningError::invalid_claim("signature must be 64 bytes"))?;

        Ok(Some(TradingKeyClaim {
            issuer: crate::types::AccountId(issuer_bytes),
            subject: crate::types::AccountId(subject_bytes),
            permissions: proto.permissions,
            max_daily_usd: proto.max_daily_usd,
            expiry_timestamp: proto.expiry_timestamp,
            nonce_sub_range_start: proto.nonce_sub_range_start,
            nonce_sub_range_end: proto.nonce_sub_range_end,
            // The decoded signature variant is a placeholder. The wire format
            // does not carry the signing algorithm. Verification does not
            // depend on the variant. Callers must not use the variant to tell
            // which algorithm signed the claim.
            signature: crate::types::Signature::Ed25519(sig_bytes),
        }))
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

#[cfg(test)]
mod verify_by_signature_kind {
    use super::*;
    use crate::types::{AccountId, PublicKey, Signature};
    use crate::SigningError;

    const NOW: u64 = 1_700_000_000;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Kind {
        Ed25519,
        Secp256k1,
        Schnorr,
    }

    const KINDS: [Kind; 3] = [Kind::Ed25519, Kind::Secp256k1, Kind::Schnorr];

    fn issuer_key(kind: Kind) -> PublicKey {
        match kind {
            Kind::Ed25519 => PublicKey::Ed25519([0xA1; 32]),
            Kind::Secp256k1 => {
                let mut bytes = [0x02; 33];
                bytes[1] = 0xB2;
                PublicKey::Secp256k1(bytes)
            }
            Kind::Schnorr => PublicKey::Schnorr([0xC3; 32]),
        }
    }

    fn signature_of(kind: Kind, nonzero: bool) -> Signature {
        let byte = if nonzero {
            match kind {
                Kind::Ed25519 => 0x11,
                Kind::Secp256k1 => 0x22,
                Kind::Schnorr => 0x33,
            }
        } else {
            0
        };
        let bytes = [byte; 64];
        match kind {
            Kind::Ed25519 => Signature::Ed25519(bytes),
            Kind::Secp256k1 => Signature::Secp256k1(bytes),
            Kind::Schnorr => Signature::Schnorr(bytes),
        }
    }

    fn issued(issuer: &PublicKey, signature: Signature) -> TradingKeyClaim {
        VcClaimBuilder::new()
            .issuer(issuer.to_account_id())
            .subject(AccountId([0x11; 32]))
            .permissions(1)
            .expiry(NOW + 3_600)
            .nonce_sub_range(0, 1)
            .signature(signature)
            .build(NOW)
            .expect("fixture claim builds")
    }

    #[test]
    fn each_issuer_key_accepts_every_nonzero_signature_variant() {
        let mut ids = KINDS.map(|kind| issuer_key(kind).to_account_id());
        ids.sort_by_key(|id| id.0);
        assert_ne!(ids[0], ids[1], "issuer account ids must be distinct");
        assert_ne!(ids[1], ids[2], "issuer account ids must be distinct");

        for issuer_kind in KINDS {
            let key = issuer_key(issuer_kind);
            for signature_kind in KINDS {
                let claim = issued(&key, signature_of(signature_kind, true));
                assert!(
                    claim.verify(NOW, &key).is_ok(),
                    "{issuer_kind:?} issuer rejected a nonzero {signature_kind:?} signature"
                );
            }
        }
    }

    #[test]
    fn each_issuer_key_rejects_a_zero_signature_of_every_variant() {
        for issuer_kind in KINDS {
            let key = issuer_key(issuer_kind);
            for signature_kind in KINDS {
                let mut claim = issued(&key, signature_of(signature_kind, true));
                claim.signature = signature_of(signature_kind, false);
                let err = claim
                    .verify(NOW, &key)
                    .expect_err("a zero signature is missing");
                assert!(
                    matches!(err, SigningError::InvalidClaim(ref msg) if msg.contains("missing signature")),
                    "{issuer_kind:?} issuer with a zero {signature_kind:?} signature returned {err:?}"
                );
            }
        }
    }

    #[cfg(feature = "cryptogram")]
    mod signed_under_its_curve {
        use super::*;
        use crate::cryptogram_crypto::{
            ecdsa_sign, ed25519_public_key, ed25519_sign, ed25519_verify, public_key_from_secret,
            recover_compressed_pubkey, schnorr_sign, schnorr_verify, schnorr_x_only_pubkey,
        };

        const ED_SEED: [u8; 32] = [0x11; 32];
        const SECP_SEED: [u8; 32] = [0x22; 32];
        const SCHNORR_SEED: [u8; 32] = [0x33; 32];

        struct Produced {
            bytes: [u8; 64],
            recovery: u8,
        }

        fn on_curve_key(kind: Kind) -> PublicKey {
            match kind {
                Kind::Ed25519 => PublicKey::Ed25519(ed25519_public_key(&ED_SEED)),
                Kind::Secp256k1 => {
                    let key = public_key_from_secret(&SECP_SEED).expect("secp256k1 seed");
                    PublicKey::Secp256k1(key.serialize())
                }
                Kind::Schnorr => {
                    PublicKey::Schnorr(schnorr_x_only_pubkey(&SCHNORR_SEED).expect("schnorr seed"))
                }
            }
        }

        fn produce(kind: Kind, digest: &[u8; 32]) -> Produced {
            match kind {
                Kind::Ed25519 => Produced {
                    bytes: ed25519_sign(digest, &ED_SEED).expect("ed25519 sign"),
                    recovery: 0,
                },
                Kind::Secp256k1 => {
                    let full = ecdsa_sign(digest, &SECP_SEED).expect("secp256k1 sign");
                    assert_eq!(full.len(), 65, "ecdsa_sign returns r || s || v");
                    let mut bytes = [0u8; 64];
                    bytes.copy_from_slice(&full[..64]);
                    Produced {
                        bytes,
                        recovery: full[64],
                    }
                }
                Kind::Schnorr => Produced {
                    bytes: schnorr_sign(digest, &SCHNORR_SEED).expect("schnorr sign"),
                    recovery: 0,
                },
            }
        }

        fn wrap(kind: Kind, bytes: [u8; 64]) -> Signature {
            match kind {
                Kind::Ed25519 => Signature::Ed25519(bytes),
                Kind::Secp256k1 => Signature::Secp256k1(bytes),
                Kind::Schnorr => Signature::Schnorr(bytes),
            }
        }

        fn secp_recovers(digest: &[u8; 32], produced: &Produced, key: &[u8; 33]) -> bool {
            let mut signature = [0u8; 65];
            signature[..64].copy_from_slice(&produced.bytes);
            let mut try_v = |v: u8| {
                signature[64] = v;
                recover_compressed_pubkey(digest, &signature).ok() == Some(*key)
            };
            if produced.recovery == 27 || produced.recovery == 28 {
                try_v(produced.recovery)
            } else {
                try_v(27) || try_v(28)
            }
        }

        fn verifies(kind: Kind, key: &PublicKey, digest: &[u8; 32], produced: &Produced) -> bool {
            match (kind, key) {
                (Kind::Ed25519, PublicKey::Ed25519(pk)) => {
                    ed25519_verify(digest, &produced.bytes, pk).unwrap_or(false)
                }
                (Kind::Secp256k1, PublicKey::Secp256k1(pk)) => secp_recovers(digest, produced, pk),
                (Kind::Schnorr, PublicKey::Schnorr(pk)) => {
                    schnorr_verify(digest, &produced.bytes, pk).unwrap_or(false)
                }
                _ => false,
            }
        }

        #[test]
        fn a_signature_valid_under_one_curve_verifies_for_every_issuer_key() {
            let keys = KINDS.map(on_curve_key);
            let mut ids = keys.clone().map(|key| key.to_account_id());
            ids.sort_by_key(|id| id.0);
            assert_ne!(
                ids[0], ids[1],
                "on-curve issuer account ids must be distinct"
            );
            assert_ne!(
                ids[1], ids[2],
                "on-curve issuer account ids must be distinct"
            );

            for (checker_index, checker) in KINDS.into_iter().enumerate() {
                let checker_key = &keys[checker_index];
                for signer in KINDS {
                    let mut claim = issued(checker_key, Signature::Ed25519([1u8; 64]));
                    let digest = claim.claim_digest();
                    let produced = produce(signer, &digest);
                    assert!(
                        verifies(signer, &keys[index(signer)], &digest, &produced),
                        "signature from {signer:?} did not verify under that curve"
                    );
                    claim.signature = wrap(signer, produced.bytes);
                    assert!(
                        claim.verify(NOW, checker_key).is_ok(),
                        "{checker:?} issuer rejected a signature from {signer:?}"
                    );
                    if signer != checker {
                        assert!(
                            !verifies(checker, checker_key, &digest, &produced),
                            "signature from {signer:?} also verified under {checker:?}"
                        );
                    }
                }
            }
        }

        fn index(kind: Kind) -> usize {
            KINDS.iter().position(|item| *item == kind).expect("kind")
        }
    }
}
