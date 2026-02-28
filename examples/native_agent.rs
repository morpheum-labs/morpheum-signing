//! Example: AI Agent signer with TradingKey + VC claim (unlimited parallelism)
//!
//! Recommended for autonomous agents, HFT, marketplaces, and high-frequency trading.

use morpheum_signing_native::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create agent signer with TradingKey
    let trading_key_seed = [99u8; 32]; // Replace with secure key
    let agent_id = AccountId([1u8; 32]); // Real AgentId from registration

    let trading_key_claim = VcClaimBuilder::new()
        .issuer(agent_id.clone())
        .subject(agent_id.clone())
        .permissions(1 << 0) // TRADE bit
        .max_daily_usd(1_000_000)
        .expiry(chrono::Utc::now().timestamp() as u64 + 86_400) // 24h
        .nonce_sub_range(1000, 2000) // isolated sub-range for parallelism
        .signature(Signature(vec![0u8; 64])) // In real code: signed by owner
        .build()?;

    let signer = AgentSigner::new(&trading_key_seed, agent_id.clone(), Some(trading_key_claim));

    // 2. Build transaction (AgentPortal hot-path recommended)
    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .memo("Test market creation from AI agent")
        .create_market("ETH-USD-PERP".to_string())
        .with_trading_key_claim(trading_key_claim) // embeds VC claim
        .sign()
        .await?;

    println!("✅ Agent Tx signed successfully with TradingKey!");
    println!("TxHash: {}", signed_tx.txhash_hex());
    println!("Nonce sub-range used: {}", signed_tx.tx.nonce.sub);

    // In real usage: submit directly to AgentPortal (sub-100µs)
    // let portal = ...;
    // portal.submit_tx(signed_tx).await?;

    Ok(())
}