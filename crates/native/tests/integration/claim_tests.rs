//! TradingKeyClaim verification, encoding, and edge-case tests.
//!
//! Covers:
//! - `verify()` (with `claim-verification` feature)
//! - `to_proto_any()` / `encode_to_vec()` deterministic encoding
//! - `claim_digest()` stability and exclusion of signature field
//! - `sub_range_size()` correctness
//! - Edge cases: boundary timestamps, max u64, wrapping

use super::common::*;
use morpheum_signing_core::{
    claim::{VcClaimBuilder, TRADING_KEY_CLAIM_TYPE_URL},
    error::SigningError,
    prelude::*,
};

// ==================== VERIFICATION TESTS ====================

#[cfg(feature = "claim-verification")]
mod verification {
    use super::*;
    use morpheum_signing_core::signer::Signer;
    use morpheum_signing_native::NativeSigner;

    #[test]
    fn test_verify_succeeds_with_matching_issuer() {
        let signer = NativeSigner::from_seed(&TEST_SEED);
        let issuer_pubkey = signer.public_key();
        let issuer_account_id = issuer_pubkey.to_account_id();
        let now = now_secs();

        let claim = VcClaimBuilder::new()
            .issuer(issuer_account_id)
            .subject(test_account_id())
            .permissions(1)
            .max_daily_usd(500_000)
            .expiry(now + 86_400)
            .nonce_sub_range(0, 1000)
            .signature(Signature::Ed25519([1u8; 64])) // non-zero dummy
            .build(now)
            .unwrap();

        let result = claim.verify(now, &issuer_pubkey);
        assert!(
            result.is_ok(),
            "verify should succeed when issuer matches pubkey"
        );
    }

    #[test]
    fn test_verify_fails_with_mismatched_issuer() {
        let signer = NativeSigner::from_seed(&TEST_SEED);
        let issuer_pubkey = signer.public_key();
        let now = now_secs();

        // Use a different account ID as issuer (won't match signer's pubkey)
        let wrong_issuer = AccountId([0xFFu8; 32]);

        let claim = VcClaimBuilder::new()
            .issuer(wrong_issuer)
            .subject(test_account_id())
            .permissions(1)
            .max_daily_usd(500_000)
            .expiry(now + 86_400)
            .nonce_sub_range(0, 1000)
            .signature(Signature::Ed25519([1u8; 64]))
            .build(now)
            .unwrap();

        let result = claim.verify(now, &issuer_pubkey);
        assert!(result.is_err());
        match result.unwrap_err() {
            SigningError::ClaimVerification(msg) if msg.contains("issuer") => {}
            e => panic!("Expected ClaimVerification issuer mismatch error, got: {e:?}"),
        }
    }

    #[test]
    fn test_verify_fails_when_expired() {
        let signer = NativeSigner::from_seed(&TEST_SEED);
        let issuer_pubkey = signer.public_key();
        let issuer_id = issuer_pubkey.to_account_id();
        let now = now_secs();

        let claim = VcClaimBuilder::new()
            .issuer(issuer_id)
            .subject(test_account_id())
            .permissions(1)
            .expiry(now + 10)
            .nonce_sub_range(0, 100)
            .signature(Signature::Ed25519([1u8; 64]))
            .build(now)
            .unwrap();

        // Verify at a time past expiry
        let result = claim.verify(now + 100, &issuer_pubkey);
        assert!(result.is_err());
        match result.unwrap_err() {
            SigningError::InvalidClaim(msg) if msg.contains("expired") => {}
            e => panic!("Expected expired error, got: {e:?}"),
        }
    }

    #[test]
    fn test_verify_with_different_key_types() {
        use morpheum_signing_native::EvmSigner;

        let evm_signer = EvmSigner::from_seed(&TEST_SEED);
        let evm_pubkey = evm_signer.public_key();
        let evm_account_id = evm_pubkey.to_account_id();
        let now = now_secs();

        let claim = VcClaimBuilder::new()
            .issuer(evm_account_id)
            .subject(test_account_id())
            .permissions(0xFF)
            .max_daily_usd(1_000_000)
            .expiry(now + 86_400)
            .nonce_sub_range(100, 200)
            .signature(Signature::Secp256k1([2u8; 64]))
            .build(now)
            .unwrap();

        let result = claim.verify(now, &evm_pubkey);
        assert!(
            result.is_ok(),
            "verify should succeed with Secp256k1 key type"
        );
    }
}

// ==================== ENCODING TESTS ====================

#[test]
fn test_encode_to_vec_is_deterministic() {
    let claim = test_trading_key_claim();
    let bytes1 = claim.encode_to_vec();
    let bytes2 = claim.encode_to_vec();
    assert_eq!(bytes1, bytes2, "encode_to_vec must be deterministic");
    assert!(!bytes1.is_empty(), "Encoded claim should not be empty");
}

#[test]
fn test_to_proto_any_has_correct_type_url() {
    let claim = test_trading_key_claim();
    let any = claim.to_proto_any();
    assert_eq!(any.type_url, TRADING_KEY_CLAIM_TYPE_URL);
    assert!(!any.value.is_empty());
}

#[test]
fn test_into_any_matches_to_proto_any() {
    let claim = test_trading_key_claim();
    let any_ref = claim.to_proto_any();
    let any_owned = claim.into_any();
    assert_eq!(any_ref.type_url, any_owned.type_url);
    assert_eq!(any_ref.value, any_owned.value);
}

// ==================== CLAIM DIGEST TESTS ====================

#[test]
fn test_claim_digest_excludes_signature() {
    let now = now_secs();

    let claim1 = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now)
        .unwrap();

    let claim2 = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([99u8; 64])) // different signature
        .build(now)
        .unwrap();

    // Digests should be identical because signature is excluded
    assert_eq!(
        claim1.claim_digest(),
        claim2.claim_digest(),
        "Digest should not include signature"
    );
}

#[test]
fn test_claim_digest_is_32_bytes() {
    let claim = test_trading_key_claim();
    let digest = claim.claim_digest();
    assert_eq!(digest.len(), 32);
}

#[test]
fn test_claim_digest_changes_with_different_fields() {
    let now = now_secs();
    let base = || {
        VcClaimBuilder::new()
            .issuer(test_account_id())
            .subject(test_account_id())
            .permissions(1)
            .max_daily_usd(1000)
            .expiry(now + 3600)
            .nonce_sub_range(0, 100)
            .signature(Signature::Ed25519([1u8; 64]))
    };

    let claim_a = base().build(now).unwrap();
    let claim_b = base().permissions(2).build(now).unwrap();
    let claim_c = base().max_daily_usd(999).build(now).unwrap();
    let claim_d = base().nonce_sub_range(0, 200).build(now).unwrap();

    assert_ne!(claim_a.claim_digest(), claim_b.claim_digest());
    assert_ne!(claim_a.claim_digest(), claim_c.claim_digest());
    assert_ne!(claim_a.claim_digest(), claim_d.claim_digest());
}

#[test]
fn test_claim_digest_is_deterministic() {
    let claim = test_trading_key_claim();
    let d1 = claim.claim_digest();
    let d2 = claim.claim_digest();
    assert_eq!(d1, d2);
}

// ==================== SUB-RANGE SIZE ====================

#[test]
fn test_sub_range_size() {
    let claim = test_trading_key_claim();
    assert_eq!(claim.sub_range_size(), 1000); // 2000 - 1000
}

#[test]
fn test_sub_range_size_saturating() {
    // When start > end (shouldn't normally happen but tests saturating behavior)
    let now = now_secs();
    let mut claim = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now)
        .unwrap();

    // Manually set invalid range to test saturating behavior
    claim.nonce_sub_range_start = 500;
    claim.nonce_sub_range_end = 100;
    assert_eq!(
        claim.sub_range_size(),
        0,
        "sub_range_size should saturate to 0"
    );
}

// ==================== BOUNDARY / EDGE CASE TESTS ====================

#[test]
fn test_claim_with_max_u64_permissions() {
    let now = now_secs();
    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(u64::MAX)
        .max_daily_usd(u64::MAX)
        .expiry(now + 3600)
        .nonce_sub_range(0, u32::MAX)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(result.is_ok(), "Max u64 values should be valid");
}

#[test]
fn test_claim_with_zero_permissions() {
    let now = now_secs();
    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(0) // no permissions
        .max_daily_usd(0)
        .expiry(now + 3600)
        .nonce_sub_range(0, 1)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    // Zero permissions is structurally valid (policy enforcement is chain-side)
    assert!(result.is_ok());
}

#[test]
fn test_claim_expiry_exactly_at_boundary() {
    let now = now_secs();

    // Expiry == now → expired (strict less-than check)
    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now) // expires exactly now
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(
        result.is_err(),
        "Expiry at exactly current time should fail"
    );

    // Expiry == now + 1 → valid
    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 1)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(
        result.is_ok(),
        "Expiry 1 second in the future should be valid"
    );
}

#[test]
fn test_claim_validate_is_called_by_build() {
    let now = now_secs();

    // Build with expired claim should fail
    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now - 100) // already expired
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(result.is_err());
}
