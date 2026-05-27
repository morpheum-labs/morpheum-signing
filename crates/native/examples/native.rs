//! Example: Native signer (local ed25519 keypair) with sequential nonce
//!
//! This example demonstrates the recommended way to sign transactions using
//! Morpheum's **native** signer (`NativeSigner`).
//!
//! - Uses the native ed25519 curve (Morpheum's default for human accounts)
//! - Supports creation from seed bytes or BIP-39 mnemonic
//! - Dynamic `SignerInfo` — correct public key proto and sign mode
//! - Sequential nonce (Sentry-compatible)
//! - Fully generic API — only `prost_types::Any` is used for messages
//! - Clean, production-grade structure and error handling

use morpheum_signing_native::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Morpheum Native Signing Example");

    // ── Option A: Create signer from a 32-byte seed ──────────────────────
    // In production: derive this from a BIP-39 mnemonic or hardware wallet.
    let seed = [42u8; 32];
    let signer_from_seed = NativeSigner::from_seed(&seed);
    println!(
        "  Signer (from seed) account: {:?}",
        signer_from_seed.account_id()
    );

    // ── Option B: Create signer from a BIP-39 mnemonic ──────────────────
    // Requires the `bip39` feature (enabled by default in `full`).
    #[cfg(feature = "bip39")]
    let signer_from_mnemonic = {
        let mnemonic = "abandon abandon abandon abandon abandon abandon \
                        abandon abandon abandon abandon abandon about";
        let signer = NativeSigner::from_mnemonic(mnemonic, "")?;
        println!(
            "  Signer (from mnemonic) account: {:?}",
            signer.account_id()
        );
        signer
    };

    // Use the mnemonic signer if available, otherwise fall back to seed signer
    #[cfg(feature = "bip39")]
    let signer = signer_from_mnemonic;
    #[cfg(not(feature = "bip39"))]
    let signer = signer_from_seed;

    // ── Create a generic protobuf message as Any ─────────────────────────
    // In real applications, serialize your message using prost or from JS/TS.
    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![], // Replace with real serialized bytes in production
    };

    // ── Build and sign using the generic fluent API ──────────────────────
    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Test market creation from native signer")
        .add_message(market_any)
        .sign()
        .await?;

    // ── Output results ───────────────────────────────────────────────────
    println!("Transaction signed successfully!");
    println!("  TxHash          : {}", signed_tx.txhash_hex());
    println!(
        "  Nonce monotonic : {}",
        signed_tx.tx.nonce.as_ref().map_or(0, |n| n.monotonic)
    );
    println!(
        "  Memo            : {}",
        signed_tx.tx.body.as_ref().map_or("", |b| &b.memo)
    );
    println!("  Raw bytes len   : {} bytes", signed_tx.raw_bytes().len());

    Ok(())
}
