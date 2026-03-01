//! Native signer flow tests (sequential nonce via Sentry).
//!
//! This test verifies the **native** signer flow using `NativeSigner` and the
//! `native()` builder. It uses only the fully generic API (`add_message` with
//! `prost_types::Any`) to stay consistent with the library's core design.

use super::common::*;
use morpheum_signing_native::prelude::*;
use prost_types::Any;
use tokio;

#[tokio::test]
pub async fn test_native_signing_flow() {
    println!("🧪 Running Native signing flow test...");

    // 1. Prepare native signer + test nonce provider
    let seed = TEST_SEED;
    let signer = NativeSigner::from_seed(&seed);
    let nonce_provider = TestNonceProvider { monotonic: 42 };

    // 2. Create a generic protobuf message as Any (example: market creation)
    // In real tests you would serialize from primitives; here we use empty bytes
    // to keep the test focused on the signing flow itself.
    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![],
    };

    // 3. Build and sign using only the generic fluent API
    let signed_tx = native(signer)                    // ← Native builder
        .chain_id("morpheum-test-1")
        .memo("Native integration test")
        .with_nonce_provider(nonce_provider)
        .add_message(market_any)                      // ← Only generic method
        .sign()
        .await
        .expect("Native signing failed");

    // 4. Assertions
    assert!(!signed_tx.raw_bytes.is_empty(), "Signed tx should not be empty");
    assert_eq!(
        signed_tx.tx.nonce.as_ref().map_or(0, |n| n.monotonic),
        42,
        "Sequential nonce not applied correctly"
    );
    assert_eq!(signed_tx.tx.nonce.as_ref().map_or(0, |n| n.sub), 0, "Native mode should use sub=0");
    assert_eq!(signed_tx.tx.body.memo, "Native integration test");
    assert_eq!(signed_tx.tx.body.messages.len(), 1, "Exactly one message should be present");

    println!("✅ Native signing flow test passed (fully generic)");
}