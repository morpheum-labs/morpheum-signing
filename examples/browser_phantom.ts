// examples/browser_phantom.ts
// Browser example using Phantom (or any Solana injected wallet)
//
// This example demonstrates the recommended way to sign transactions
// using the Morpheum Signing SDK in the browser with Phantom Wallet.
//
// Key capabilities demonstrated:
// - Factory-method wallet connection (async, connects to window.phantom.solana)
// - Dynamic SignerInfo (ed25519 public key + SIGN_MODE_ED25519)
// - Fully generic message API (type_url + encoded bytes)
// - Optional TradingKeyClaim for agent delegation
// - Rich TypeScript type definitions

import {
    TxBuilderWasm,
    VcClaimBuilder,
    set_panic_hook,
    type SignedTx
} from '@morpheum/signing';

async function main() {
    console.log("Morpheum Phantom Signing Example");

    // Enable better panic messages in the browser console
    set_panic_hook();

    // ── Basic Phantom transaction ───────────────────────────────────────

    // Create builder configured for Phantom / Solana wallets.
    // This connects to window.phantom.solana, requests wallet access,
    // and caches the ed25519 public key (32 bytes).
    const builder = TxBuilderWasm.newPhantom()
        .chain_id("morpheum-test-1")
        .memo("Market creation from Phantom Wallet");

    // Generic message example (market creation)
    // In real applications, encode your protobuf message as Uint8Array bytes
    const marketMsgBytes = new Uint8Array([]); // Replace with real protobuf bytes

    try {
        // Sign using the fully generic API.
        // The SignerInfo will contain:
        //   - public_key: /morpheum.crypto.ed25519.PubKey (32 bytes)
        //   - mode_info: SIGN_MODE_ED25519
        const signedTx = await builder
            .add_message(
                "type.googleapis.com/market.v1.MsgCreateMarketRequest",
                marketMsgBytes
            )
            .sign();

        console.log("Transaction signed successfully with Phantom!");
        console.log("  TxHash          :", signedTx.txhash);
        console.log("  Raw bytes length:", signedTx.raw_bytes.length);

        // In a real dApp you would now broadcast signedTx.raw_bytes
        // to a Sentry node via gRPC or REST.

    } catch (error) {
        console.error("Signing failed:", error);
    }

    // ── Phantom transaction with agent claim ────────────────────────────

    try {
        const nowSecs = Math.floor(Date.now() / 1000);

        // Build a TradingKeyClaim for agent delegation
        const claim = new VcClaimBuilder()
            .issuer(new Uint8Array(32).fill(1))
            .subject(new Uint8Array(32).fill(2))
            .permissions(0x01)                       // TRADE permission
            .maxDailyUsd(50_000)                     // $50k daily limit
            .expiry(nowSecs + 86_400)                // 24 hours
            .nonceSubRange(100, 200)                 // 100 parallel operations
            .signature(new Uint8Array(64).fill(1), "ed25519")
            .build(nowSecs);

        console.log("TradingKeyClaim built:", claim.proto_any_type_url);

        // Attach claim and sign
        const signedTxWithClaim = await TxBuilderWasm.newPhantom()
            .chain_id("morpheum-test-1")
            .memo("Agent trade via Phantom")
            .withClaim(claim)
            .add_message(
                "type.googleapis.com/market.v1.MsgCreateMarketRequest",
                marketMsgBytes
            )
            .sign();

        console.log("Transaction signed with embedded claim!");
        console.log("  TxHash:", signedTxWithClaim.txhash);

    } catch (error) {
        console.error("Claim signing failed:", error);
    }
}

main();
