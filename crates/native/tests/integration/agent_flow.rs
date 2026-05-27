//! Agent signer flow tests (TradingKey + VC claim with isolated nonce sub-range).
//!
//! Verifies the full agent signing flow using `AgentSigner` and the `agent()` builder.

use super::common::*;
use morpheum_signing_native::prelude::*;

#[tokio::test]
pub async fn test_agent_signing_flow() {
    let trading_key_seed = [99u8; 32];
    let agent_id = test_account_id();
    let trading_key_claim = test_trading_key_claim();

    let signer = AgentSigner::new(&trading_key_seed, agent_id, Some(trading_key_claim.clone()));

    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![],
    };

    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .memo("Agent integration test with TradingKey + VC")
        .add_message(market_any)
        .with_trading_key_claim(trading_key_claim)
        .sign()
        .await
        .expect("Agent signing failed");

    assert!(
        !signed_tx.raw_bytes.is_empty(),
        "Signed tx should not be empty"
    );
    assert_eq!(
        signed_tx.tx.nonce.as_ref().map_or(0, |n| n.sub),
        0,
        "Default nonce sub should be 0 when no nonce provider is set"
    );
    assert_eq!(
        signed_tx.tx.body.as_ref().unwrap().memo,
        "Agent integration test with TradingKey + VC"
    );
    assert_eq!(signed_tx.tx.body.as_ref().unwrap().messages.len(), 1);
}

#[tokio::test]
pub async fn test_agent_without_claim() {
    let signer = AgentSigner::new(&[77u8; 32], test_account_id(), None);

    let msg = Any {
        type_url: "type.googleapis.com/test.v1.Msg".to_string(),
        value: vec![],
    };

    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .add_message(msg)
        .sign()
        .await
        .expect("Agent without claim should succeed");

    assert!(!signed_tx.raw_bytes.is_empty());
}

#[tokio::test]
pub async fn test_agent_different_seeds_produce_different_signatures() {
    let signer1 = AgentSigner::new(&[1u8; 32], test_account_id(), None);
    let signer2 = AgentSigner::new(&[2u8; 32], test_account_id(), None);

    let msg = || Any {
        type_url: "type.googleapis.com/test.v1.Msg".to_string(),
        value: vec![42],
    };

    let tx1 = agent(signer1)
        .chain_id("morpheum-test-1")
        .add_message(msg())
        .sign()
        .await
        .expect("Sign 1 failed");

    let tx2 = agent(signer2)
        .chain_id("morpheum-test-1")
        .add_message(msg())
        .sign()
        .await
        .expect("Sign 2 failed");

    assert_ne!(
        tx1.tx.signatures, tx2.tx.signatures,
        "Different keys must produce different sigs"
    );
}
