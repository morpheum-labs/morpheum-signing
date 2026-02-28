//! Example: Human signer (local keypair) with sequential nonce (Sentry compatibility)
//!
//! This is the recommended example for CLI tools, bots, or MetaMask-style human users.

use morpheum_signing_native::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a human signer from a seed (in production use secure mnemonic / key store)
    let seed = [42u8; 32]; // Replace with real secure seed
    let signer = HumanSigner::from_seed(&seed);

    // 2. Build transaction using fluent API
    let signed_tx = human(signer)  // convenience constructor from native
        .chain_id("morpheum-test-1")
        .memo("Test market creation from human signer")
        .create_market("BTC-USD-PERP".to_string())  // real MsgCreateMarketRequest in full version
        .sign()
        .await?;

    println!("✅ Human Tx signed successfully!");
    println!("TxHash: {}", signed_tx.txhash_hex());
    println!("Nonce monotonic: {}", signed_tx.tx.nonce.monotonic);

    // In real usage: send to Sentry node via gRPC
    // let client = ...;
    // client.submit_tx(signed_tx).await?;

    Ok(())
}