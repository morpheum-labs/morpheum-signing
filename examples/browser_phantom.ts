// examples/browser_phantom.ts
// Example: Browser frontend using Phantom (Solana injected wallet)

import { TxBuilderWasm, set_panic_hook } from '@morpheum/signing';

async function main() {
    set_panic_hook();

    const builder = new TxBuilderWasm();

    // In full version: use PhantomAdapter via web-sys
    // For demo we use basic builder (Phantom adapter added in full version)
    const signedTx = await builder
        .chain_id("morpheum-test-1")
        .memo("Market creation from Phantom wallet")
        .create_market("SOL-USD-PERP")
        .sign();

    console.log("✅ Phantom Tx signed!");
    console.log("TxHash:", signedTx.txhash);

    // Submit to AgentPortal or Sentry
}

main().catch(console.error);