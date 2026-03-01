//! Agent signer flow tests (TradingKey + VC claim with isolated nonce sub-range).
//!
//! This test verifies the full agent signing flow using `AgentSigner` and the
//! `agent()` builder. It demonstrates VC delegation, nonce sub-range isolation,
//! and uses only the fully generic API (`add_message` with `prost_types::Any`).

use super::common::*;
use morpheum_signing_native::prelude::*;
use prost_types::Any;
use tokio;

#[tokio::test]
pub async fn test_agent_signing_flow() {
    println!("🧪 Running Agent signing flow test...");

    // 1. Prepare agent signer with TradingKey + VC claim
    let trading_key_seed = [99u8; 32];
    let agent_id = test_account_id();
    let trading_key_claim = test_trading_key_claim();

    let signer = AgentSigner::new(&trading_key_seed, agent_id.clone(), Some(trading_key_claim.clone()));

    // 2. Create a generic protobuf message as Any (example: market creation)
    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![], // Real serialized bytes would be used in production
    };

    // 3. Build and sign using only the generic fluent API
    let signed_tx = agent(signer)                     // ← Agent builder
        .chain_id("morpheum-test-1")
        .memo("Agent integration test with TradingKey + VC")
        .add_message(market_any)
        .with_trading_key_claim(trading_key_claim)
        .sign()
        .await
        .expect("Agent signing failed");

    // 4. Assertions
    assert!(!signed_tx.raw_bytes.is_empty(), "Signed tx should not be empty");

    // Agent flow should use the nonce sub-range from the claim
    assert_eq!(
        signed_tx.tx.nonce.as_ref().map_or(0, |n| n.sub),
        1000,
        "Nonce sub-range from TradingKeyClaim was not applied"
    );

    assert_eq!(signed_tx.tx.body.memo, "Agent integration test with TradingKey + VC");
    assert_eq!(signed_tx.tx.body.messages.len(), 1, "Exactly one message should be present");

    println!("✅ Agent signing flow test passed (fully generic with VC delegation)");
}