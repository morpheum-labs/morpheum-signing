// examples/browser_metamask.ts
// Browser example using MetaMask (or any EVM injected wallet)
//
// This example demonstrates the recommended way to sign transactions
// using the Morpheum Signing SDK in the browser with MetaMask.
//
// Key capabilities demonstrated:
// - Factory-method wallet connection (async, connects to window.ethereum)
// - Dynamic SignerInfo (secp256k1 public key + SIGN_MODE_SECP256K1)
// - Fully generic message API (type_url + encoded bytes)
// - Optional TradingKeyClaim attachment for agent delegation
// - Rich TypeScript type definitions

import {
    TxBuilderWasm,
    VcClaimBuilder,
    set_panic_hook,
    type SignedTx,
    type TradingKeyClaimInput
} from '@morpheum/signing';

async function main() {
    console.log("Morpheum MetaMask Signing Example");

    // Enable better panic messages in the browser console
    set_panic_hook();

    // ── Basic MetaMask transaction ──────────────────────────────────────

    // Create builder configured for MetaMask / EVM wallets.
    // This connects to window.ethereum, requests account access,
    // and caches the EVM address + secp256k1 public key.
    const builder = TxBuilderWasm.newMetamask()
        .chain_id("morpheum-test-1")
        .memo("Market creation from MetaMask");

    // Generic message example (market creation)
    // In real applications, encode your protobuf message as Uint8Array bytes
    const marketMsgBytes = new Uint8Array([]); // Replace with real protobuf bytes

    try {
        // Sign using the fully generic API.
        // The SignerInfo will contain:
        //   - public_key: /morpheum.crypto.secp256k1.PubKey (33 bytes, compressed)
        //   - mode_info: SIGN_MODE_SECP256K1
        const signedTx = await builder
            .add_message(
                "type.googleapis.com/market.v1.MsgCreateMarketRequest",
                marketMsgBytes
            )
            .sign();

        console.log("Transaction signed successfully with MetaMask!");
        console.log("  TxHash          :", signedTx.txhash);
        console.log("  Raw bytes length:", signedTx.raw_bytes.length);

        // In a real dApp you would now broadcast signedTx.raw_bytes
        // to a Sentry node via gRPC or REST.

    } catch (error) {
        console.error("Signing failed:", error);
    }

    // ── MetaMask transaction with TradingKeyClaim ───────────────────────

    try {
        // Build a TradingKeyClaim using the fluent builder
        const nowSecs = Math.floor(Date.now() / 1000);

        const claim = new VcClaimBuilder()
            .issuer(new Uint8Array(32).fill(1))     // 32-byte issuer AccountId
            .subject(new Uint8Array(32).fill(2))     // 32-byte subject AccountId
            .permissions(0x01)                       // TRADE permission
            .maxDailyUsd(100_000)                    // $100k daily limit
            .expiry(nowSecs + 86_400)                // 24 hours from now
            .nonceSubRange(1000, 2000)               // 1000 parallel operations
            .signature(new Uint8Array(64).fill(1), "ed25519")  // Issuer's signature
            .build(nowSecs);

        console.log("TradingKeyClaim built successfully");
        console.log("  Proto type URL:", claim.proto_any_type_url);

        // Attach claim and sign
        const signedTxWithClaim = await TxBuilderWasm.newMetamask()
            .chain_id("morpheum-test-1")
            .memo("Agent delegation via MetaMask")
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
