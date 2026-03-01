//! Example: AI Agent signer with TradingKey + VC claim (unlimited parallelism)
//!
//! This example demonstrates the recommended pattern for autonomous AI agents:
//! - Uses `AgentSigner` with a `TradingKeyClaim` (VC delegation)
//! - Claim is validated and embedded in `SignerInfo.signing_options`
//! - Isolated nonce sub-range for high parallelism
//! - Dynamic `SignerInfo` with Agent public key type and Ed25519 sign mode
//! - Fully generic API — only `prost_types::Any` is used for messages
//!
//! This is the standard pattern for production agent infrastructure.

use morpheum_signing_native::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Morpheum AI Agent Signing Example");

    // 1. Prepare Agent ID and TradingKey seed
    let agent_id = AccountId([1u8; 32]);       // In production: derive from registered Agent DID
    let trading_key_seed = [99u8; 32];         // In production: securely stored or derived

    // 2. Build TradingKeyClaim (VC delegation with nonce sub-range)
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    let trading_key_claim = VcClaimBuilder::new()
        .issuer(agent_id.clone())
        .subject(agent_id.clone())
        .permissions(1 << 0)                       // TRADE permission
        .max_daily_usd(1_000_000)
        .expiry(now_secs + 86_400)                 // 24 hours from now
        .nonce_sub_range(1000, 2000)               // Allows up to 1000 parallel operations
        .signature(Signature::Ed25519([1u8; 64]))  // In production: real issuer signature
        .build(now_secs)?;

    println!("  Claim sub-range size: {}", trading_key_claim.sub_range_size());

    // 3. Create AgentSigner with the claim
    let signer = AgentSigner::new(&trading_key_seed, agent_id, Some(trading_key_claim.clone()));

    // 4. Create a generic protobuf message as Any
    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![], // Replace with real serialized bytes in production
    };

    // 5. Build and sign using the generic fluent API
    //    The claim is automatically embedded in SignerInfo.signing_options
    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .memo("Test market creation from AI agent with TradingKey + VC")
        .add_message(market_any)
        .with_trading_key_claim(trading_key_claim)
        .sign()
        .await?;

    // 6. Output results
    println!("Agent transaction signed successfully with TradingKey + VC claim!");
    println!("  TxHash          : {}", signed_tx.txhash_hex());
    println!("  Nonce sub-range : {}", signed_tx.tx.nonce.as_ref().map_or(0, |n| n.sub));
    println!("  Memo            : {}", signed_tx.tx.body.as_ref().map_or("", |b| &b.memo));
    println!("  Raw bytes len   : {} bytes", signed_tx.raw_bytes().len());

    Ok(())
}
