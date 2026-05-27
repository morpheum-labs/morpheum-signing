//! Native signer flow tests (sequential nonce via Sentry).
//!
//! Verifies the **native** signer flow using `NativeSigner` and the `native()` builder.

use super::common::*;
use morpheum_signing_native::prelude::*;

#[tokio::test]
pub async fn test_native_signing_flow() {
    let signer = NativeSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 42 };

    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![],
    };

    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Native integration test")
        .with_nonce_provider(nonce_provider)
        .add_message(market_any)
        .sign()
        .await
        .expect("Native signing failed");

    assert!(
        !signed_tx.raw_bytes.is_empty(),
        "Signed tx should not be empty"
    );
    assert_eq!(
        signed_tx.tx.nonce.as_ref().map_or(0, |n| n.monotonic),
        42,
        "Sequential nonce not applied correctly"
    );
    assert_eq!(signed_tx.tx.nonce.as_ref().map_or(0, |n| n.sub), 0);
    assert_eq!(
        signed_tx.tx.body.as_ref().unwrap().memo,
        "Native integration test"
    );
    assert_eq!(signed_tx.tx.body.as_ref().unwrap().messages.len(), 1);
}

#[tokio::test]
pub async fn test_native_default_nonce_fallback() {
    let signer = NativeSigner::from_seed(&TEST_SEED);

    let msg = Any {
        type_url: "type.googleapis.com/test.v1.Msg".to_string(),
        value: vec![1, 2, 3],
    };

    // No nonce provider → falls back to zero nonce
    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .add_message(msg)
        .sign()
        .await
        .expect("Should succeed with default nonce");

    assert_eq!(
        signed_tx
            .tx
            .nonce
            .as_ref()
            .map_or(u64::MAX, |n| n.monotonic),
        0
    );
}

#[tokio::test]
pub async fn test_native_multiple_messages() {
    let signer = NativeSigner::from_seed(&TEST_SEED);

    let msg1 = Any {
        type_url: "type.googleapis.com/test.v1.MsgA".to_string(),
        value: vec![1],
    };
    let msg2 = Any {
        type_url: "type.googleapis.com/test.v1.MsgB".to_string(),
        value: vec![2],
    };
    let msg3 = Any {
        type_url: "type.googleapis.com/test.v1.MsgC".to_string(),
        value: vec![3],
    };

    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .add_message(msg1)
        .add_message(msg2)
        .add_message(msg3)
        .sign()
        .await
        .expect("Multiple messages should succeed");

    assert_eq!(signed_tx.tx.body.as_ref().unwrap().messages.len(), 3);
}

#[tokio::test]
pub async fn test_native_deterministic_signatures() {
    let signer1 = NativeSigner::from_seed(&TEST_SEED);
    let signer2 = NativeSigner::from_seed(&TEST_SEED);
    let nonce1 = TestNonceProvider { monotonic: 1 };
    let nonce2 = TestNonceProvider { monotonic: 1 };

    let msg = || Any {
        type_url: "type.googleapis.com/test.v1.Msg".to_string(),
        value: vec![42],
    };

    let tx1 = native(signer1)
        .chain_id("morpheum-test-1")
        .with_nonce_provider(nonce1)
        .add_message(msg())
        .sign()
        .await
        .expect("Sign 1 failed");

    let tx2 = native(signer2)
        .chain_id("morpheum-test-1")
        .with_nonce_provider(nonce2)
        .add_message(msg())
        .sign()
        .await
        .expect("Sign 2 failed");

    // Same key + same inputs → identical signatures
    assert_eq!(tx1.raw_bytes, tx2.raw_bytes, "Deterministic signing failed");
    assert_eq!(tx1.tx.signatures, tx2.tx.signatures);
}
