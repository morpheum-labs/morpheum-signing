//! Error cases and negative path tests for the Morpheum Signing SDK.
//!
//! Verifies that all expected error conditions are handled correctly with
//! clear, actionable error messages and proper error variants.

use super::common::*;
use morpheum_signing_core::{error::SigningError, prelude::*};
use morpheum_signing_native::prelude::*;

#[tokio::test]
pub async fn test_builder_rejects_empty_messages() {
    let signer = NativeSigner::from_seed(&TEST_SEED);

    let result = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Test without messages")
        .sign()
        .await;

    assert!(
        result.is_err(),
        "Builder should fail when no messages are added"
    );
    match result.unwrap_err() {
        SigningError::Signing(msg) if msg.contains("message") || msg.contains("empty") => {}
        e => panic!("Expected Signing error about missing messages, got: {e:?}"),
    }
}

#[tokio::test]
pub async fn test_expired_trading_key_claim() {
    let now = now_secs();

    let expired_claim = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .max_daily_usd(1000)
        .expiry(now + 10) // will be valid for build
        .nonce_sub_range(1000, 2000)
        .signature(Signature::Ed25519([1u8; 64])) // non-zero dummy
        .build(now)
        .unwrap();

    // Now pretend the claim is expired by creating one with past expiry
    let expired_claim = TradingKeyClaim {
        expiry_timestamp: 1_000_000, // far in the past
        ..expired_claim
    };

    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(expired_claim.clone()));

    let result = agent(signer)
        .chain_id("morpheum-test-1")
        .add_message(Any {
            type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
            value: vec![],
        })
        .with_trading_key_claim(expired_claim)
        .sign()
        .await;

    assert!(result.is_err(), "Expired claim should be rejected");
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("expired") => {}
        e => panic!("Expected InvalidClaim with 'expired', got: {e:?}"),
    }
}

#[test]
fn test_invalid_nonce_sub_range_in_claim() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .nonce_sub_range(2000, 1000) // start > end
        .expiry(now + 3600)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(result.is_err(), "Invalid nonce sub-range should fail");
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("nonce") => {}
        e => panic!("Expected InvalidClaim about nonce range, got: {e:?}"),
    }
}

#[test]
fn test_equal_nonce_sub_range_is_invalid() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .nonce_sub_range(1000, 1000) // start == end
        .expiry(now + 3600)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(
        result.is_err(),
        "Equal nonce sub-range should fail (empty range)"
    );
}

#[test]
fn test_missing_issuer_in_claim_builder() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(result.is_err());
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("issuer") => {}
        e => panic!("Expected error about missing issuer, got: {e:?}"),
    }
}

#[test]
fn test_missing_subject_in_claim_builder() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(result.is_err());
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("subject") => {}
        e => panic!("Expected error about missing subject, got: {e:?}"),
    }
}

#[test]
fn test_missing_signature_in_claim_builder() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .build(now);

    assert!(result.is_err());
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("signature") => {}
        e => panic!("Expected error about missing signature, got: {e:?}"),
    }
}

#[test]
fn test_missing_expiry_in_claim_builder() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([1u8; 64]))
        .build(now);

    assert!(result.is_err());
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("expiry") => {}
        e => panic!("Expected error about missing expiry, got: {e:?}"),
    }
}

#[test]
fn test_zero_signature_is_rejected() {
    let now = now_secs();

    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .expiry(now + 3600)
        .nonce_sub_range(0, 100)
        .signature(Signature::Ed25519([0u8; 64])) // all-zero = "missing"
        .build(now);

    assert!(
        result.is_err(),
        "All-zero signature should be treated as missing"
    );
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("signature") => {}
        e => panic!("Expected error about missing signature, got: {e:?}"),
    }
}

#[test]
fn test_error_display_messages() {
    let err = SigningError::invalid_key("bad mnemonic");
    assert!(err.to_string().contains("bad mnemonic"));

    let err = SigningError::wallet_adapter("MetaMask rejected");
    assert!(err.to_string().contains("MetaMask rejected"));

    let err = SigningError::signing("payload too large");
    assert!(err.to_string().contains("payload too large"));

    let err = SigningError::invalid_claim("expired");
    assert!(err.to_string().contains("expired"));

    let err = SigningError::custom("rare edge case");
    assert!(err.to_string().contains("rare edge case"));
}
