//! Example: Native signer (local ed25519 keypair) with sequential nonce
//!
//! This example demonstrates the recommended way to sign transactions using
//! Morpheum's **native** signer (`NativeSigner`).
//!
//! - Uses the native ed25519 curve (Morpheum's default for human accounts)
//! - Sequential nonce (Sentry-compatible, MetaMask-style behavior)
//! - Fully generic API — only `prost_types::Any` is used for messages
//! - Clean, production-grade structure and error handling

use morpheum_signing_native::prelude::*;
use prost_types::Any;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Morpheum Native Signing Example");

    // 1. Create native signer from secure seed
    // In production: derive this from a BIP-39 mnemonic or hardware wallet
    let seed = [42u8; 32];
    let signer = NativeSigner::from_seed(&seed);

    // 2. Create a generic protobuf message as Any
    // In real applications, serialize your message using prost or from JS/TS.
    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![], // ← Replace with real serialized bytes in production
    };

    // 3. Build and sign using the generic fluent API
    let signed_tx = native(signer)                    // ← Native builder
        .chain_id("morpheum-test-1")
        .memo("Test market creation from native signer")
        .add_message(market_any)
        .sign()
        .await?;

    // 4. Output results
    println!("✅ Native transaction signed successfully!");
    println!("   TxHash           : {}", signed_tx.txhash_hex());
    println!("   Nonce monotonic  : {}", signed_tx.tx.nonce.as_ref().map_or(0, |n| n.monotonic));
    println!("   Memo             : {}", signed_tx.tx.body.memo);

    Ok(())
}