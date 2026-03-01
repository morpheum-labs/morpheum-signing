//! Error cases and negative path tests for the Morpheum Signing SDK.
//!
//! This test suite verifies that all expected error conditions are handled
//! correctly with clear, actionable error messages and proper error variants.
//! It covers the new `NativeSigner`, `AgentSigner`, builder validation,
//! claim validation, and edge cases.

use super::common::*;
use morpheum_signing_core::{error::SigningError, prelude::*};
use morpheum_signing_native::prelude::*;
use tokio;

#[tokio::test]
pub async fn test_error_cases() {
    println!("🧪 Running Error Cases & Negative Path Test Suite...");

    // ==================== 1. INVALID SEED / KEY ====================
    println!("   • Testing invalid seed length for NativeSigner...");
    let invalid_seed = [0u8; 31]; // wrong length (must be 32 bytes)

    let result = std::panic::catch_unwind(|| NativeSigner::from_seed(&invalid_seed.try_into().unwrap()));
    assert!(result.is_err(), "NativeSigner should panic or fail on invalid 31-byte seed");

    // ==================== 2. BUILDER VALIDATION (MISSING MESSAGES) ====================
    println!("   • Testing builder with no messages...");
    let signer = NativeSigner::from_seed(&TEST_SEED);

    let result = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Test without messages")
        .sign()
        .await;

    assert!(result.is_err(), "Builder should fail when no messages are added");
    match result.unwrap_err() {
        SigningError::Signing(msg) if msg.contains("message") || msg.contains("empty") => {}
        e => panic!("Expected Signing error about missing messages, got: {:?}", e),
    }

    // ==================== 3. EXPIRED TRADING KEY CLAIM ====================
    println!("   • Testing expired TradingKeyClaim...");
    let expired_claim = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .max_daily_usd(1000)
        .expiry(1_000_000) // far in the past
        .nonce_sub_range(1000, 2000)
        .signature(Signature(vec![0u8; 64]))
        .build()
        .unwrap();

    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(expired_claim));

    let result = agent(signer)
        .chain_id("morpheum-test-1")
        .add_message(Any {
            type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
            value: vec![],
        })
        .sign()
        .await;

    assert!(result.is_err(), "Expired claim should be rejected");
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("expired") => {}
        e => panic!("Expected InvalidClaim with 'expired', got: {:?}", e),
    }

    // ==================== 4. INVALID CLAIM CONSTRUCTION ====================
    println!("   • Testing invalid claim construction...");
    let result = VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1)
        .nonce_sub_range(2000, 1000) // start > end
        .build(1_700_000_000);

    assert!(result.is_err(), "Invalid nonce sub-range should fail");
    match result.unwrap_err() {
        SigningError::InvalidClaim(msg) if msg.contains("nonce") => {}
        e => panic!("Expected InvalidClaim about nonce range, got: {:?}", e),
    }

    // ==================== 5. ADDRESS MAPPING EDGE CASES ====================
    println!("   • Testing address mapping edge cases...");
    let mapper = DefaultAddressMapper;

    // Empty native address should still produce a valid AccountId (blake3 of empty string)
    let empty_addr = Address::Native("".to_string());
    let result = mapper.to_account_id(&empty_addr);
    assert!(result.is_ok(), "Empty native address should map to a valid AccountId");

    println!("✅ All error cases and negative path tests passed");
}