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
    setPanicHook,
    type SignedTx,
    type TradingKeyClaimInput
} from '@morpheum/signing';

async function main() {
    console.log("Morpheum MetaMask Signing Example");

    // Enable better panic messages in the browser console
    setPanicHook();

    // ── Basic MetaMask transaction ──────────────────────────────────────

    // Create builder configured for MetaMask / EVM wallets.
    // This connects to window.ethereum, requests account access,
    // and caches the EVM address + secp256k1 public key.
    // The target chain's 32-byte genesis hash, bound into every signature so
    // it cannot be replayed onto another chain sharing the chain ID. Take it
    // from operator configuration, never from the node you submit to; sign()
    // refuses a transaction built without one.
    const genesisHash = new Uint8Array(32); // Replace with the configured genesis hash

    const builder = (await TxBuilderWasm.newMetamask())
        .chainId("morpheum-test-1")
        .withGenesisHash(genesisHash)
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
            .addMessage(
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
            .permissions(0x01n)                      // TRADE permission
            .maxDailyUsd(100_000n)                    // $100k daily limit
            .expiry(BigInt(nowSecs + 86_400))                // 24 hours from now
            .nonceSubRange(1000, 2000)               // 1000 parallel operations
            .signature(new Uint8Array(64).fill(1), "ed25519")  // Issuer's signature
            .build(BigInt(nowSecs));

        console.log("TradingKeyClaim built successfully");
        console.log("  Proto type URL:", claim.proto_any_type_url);

        // Attach claim and sign
        const signedTxWithClaim = await (await TxBuilderWasm.newMetamask())
            .chainId("morpheum-test-1")
            .withGenesisHash(genesisHash)
            .memo("Agent delegation via MetaMask")
            .withClaim(claim)
            .addMessage(
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
