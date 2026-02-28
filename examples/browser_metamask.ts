// examples/browser_metamask.ts
// Example: Browser frontend using MetaMask (EVM injected wallet)

import { TxBuilderWasm, set_panic_hook } from '@morpheum/signing'; // after wasm-pack

async function main() {
    set_panic_hook(); // Better error messages in browser console

    const builder = new TxBuilderWasm();

    // In real MetaMask integration, you would use web-sys adapter
    // For demo we use the basic builder (MetaMask adapter added in full version)
    const signedTx = await builder
        .chain_id("morpheum-test-1")
        .memo("Market creation from MetaMask")
        .create_market("BTC-USD-PERP")
        .sign();

    console.log("✅ MetaMask Tx signed!");
    console.log("TxHash:", signedTx.txhash);
    console.log("Raw bytes length:", signedTx.raw_bytes.length);

    // Send to Sentry node via fetch / gRPC
    // await fetch('/tx/v1/submit', { method: 'POST', body: JSON.stringify(signedTx) });
}

main().catch(console.error);