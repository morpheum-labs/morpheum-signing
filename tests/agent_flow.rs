//! Agent signer flow tests (Portal + TradingKeyClaim).

use super::common::*;
use morpheum_signing_native::prelude::*;

#[tokio::test]
pub async fn test_agent_signing_flow() {
    let trading_key_seed = [99u8; 32];
    let agent_id = test_account_id();
    let claim = test_trading_key_claim();

    let signer = AgentSigner::new(&trading_key_seed, agent_id.clone(), Some(claim.clone()));

    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .memo("Agent integration test")
        .with_trading_key_claim(claim)
        .create_market("ETH-USD-PERP".to_string())
        .sign()
        .await
        .expect("Agent signing failed");

    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(signed_tx.tx.nonce.sub, 1000); // from claim sub-range
    assert_eq!(signed_tx.tx.memo, "Agent integration test");

    println!("✅ Agent signing flow test passed");
}