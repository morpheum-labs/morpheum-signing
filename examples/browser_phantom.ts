// examples/browser_phantom.ts
// Browser example using Phantom (or any Solana injected wallet)
//
// This example demonstrates the recommended way to sign transactions
// using the Morpheum Signing SDK in the browser with Phantom Wallet.
// It uses the new factory method and the fully generic API.

import { TxBuilderWasm, set_panic_hook } from '@morpheum/signing';

async function main() {
    console.log("🚀 Morpheum Phantom Signing Example");

    // Enable better panic messages in the browser console
    set_panic_hook();

    // Create builder configured for Phantom / Solana wallets
    const builder = TxBuilderWasm.newPhantom()
        .chain_id("morpheum-test-1")
        .memo("Market creation from Phantom Wallet");

    // Generic message example (market creation)
    // In real applications, serialize your protobuf message as a plain JS object
    const marketMsg = {
        from_address: "solana_address_here",   // Phantom/Solana address
        base_asset_index: 1,
        quote_asset_index: 2,
        market_type: 0,
        orderbook_type: "clob",
        // ... add other fields as needed
    };

    try {
        // Sign using the fully generic API
        const signedTx = await builder
            .add_message(
                "type.googleapis.com/market.v1.MsgCreateMarketRequest",
                marketMsg
            )
            .sign();

        console.log("✅ Transaction signed successfully with Phantom!");
        console.log("   TxHash          :", signedTx.txhash);
        console.log("   Nonce sub-range :", signedTx.tx?.nonce?.sub);
        console.log("   Raw bytes length:", signedTx.raw_bytes.length);

        // In a real dApp you would now broadcast signedTx.raw_bytes
        // to a Sentry node or AgentPortal.

    } catch (error) {
        console.error("❌ Signing failed:", error);
    }
}

main();