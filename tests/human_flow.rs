//! Human signer flow tests (Sentry sequential nonce).

use super::common::*;
use morpheum_signing_native::prelude::*;

#[tokio::test]
pub async fn test_human_signing_flow() {
    let signer = HumanSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 42 };

    let signed_tx = human(signer)
        .chain_id("morpheum-test-1")
        .memo("Human integration test")
        .with_nonce_provider(nonce_provider)
        .create_market("BTC-USD-PERP".to_string())
        .sign()
        .await
        .expect("Human signing failed");

    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(signed_tx.tx.nonce.monotonic, 42);
    assert_eq!(signed_tx.tx.nonce.sub, 0); // sequential mode
    assert_eq!(signed_tx.tx.memo, "Human integration test");

    println!("✅ Human signing flow test passed");
}