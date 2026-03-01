# morpheum-signing

[![Crates.io](https://img.shields.io/crates/v/morpheum-signing.svg)](https://crates.io/crates/morpheum-signing)
[![docs.rs](https://img.shields.io/docsrs/morpheum-signing)](https://docs.rs/morpheum-signing)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)

**Universal multi-chain signing SDK for Morpheum** — the official library for humans and AI agents.

Sign transactions from **MetaMask, Phantom, Taproot, native keys**, and **TradingKey + VC delegation** for autonomous agents — all with a single, elegant fluent API.

---

## ✨ Features

- **Unified API** — One `TxBuilder` for all use cases
- **Multi-chain support** — Native, EVM (MetaMask), Solana (Phantom), Bitcoin Taproot
- **Agent-first design** — `AgentSigner` with `TradingKeyClaim` and isolated nonce sub-ranges (unlimited parallelism)
- **Native-first** — `NativeSigner` for Morpheum's native ed25519 accounts
- **Dual target** — Native Rust (CLI, bots, agents) + WASM/TypeScript (browser frontends)
- **Proto-centric** — Produces exact Morpheum `Tx`, `SignDoc`, `TxRaw`, `Nonce`
- **Zero-copy & secure** — `zeroize`, `secrecy`, `no_std` core
- **Production ready** — Full test suite, excellent examples, clear error messages

---

## 📦 Installation

**Rust (native / CLI / bots / agents)**

```toml
# Cargo.toml
[dependencies]
morpheum-signing = { version = "0.1", features = ["full"] }
```

**Browser / TypeScript (React, Vue, Svelte, Next.js, etc.)**

```bash
npm install @morpheum/signing
```

---

## 🚀 Quick Start

### Native (Morpheum native signer)

```rust
use morpheum_signing_native::prelude::*;

let signer = NativeSigner::from_seed(&[42u8; 32]);

let signed_tx = native(signer)
    .chain_id("morpheum-test-1")
    .memo("Test market from native signer")
    .add_message(market_any)        // fully generic
    .sign()
    .await?;
```

### AI Agent (TradingKey + VC delegation)

```rust
let claim = VcClaimBuilder::new()
    .issuer(agent_id.clone())
    .subject(agent_id.clone())
    .permissions(1 << 0) // TRADE
    .nonce_sub_range(1000, 2000)
    .build()?;

let signed_tx = agent(signer)
    .with_trading_key_claim(claim)
    .add_message(market_any)
    .sign()
    .await?;
```

### Browser (MetaMask)

```ts
import { TxBuilderWasm, set_panic_hook } from '@morpheum/signing';

set_panic_hook();

const signedTx = await TxBuilderWasm.newMetamask()
    .chain_id("morpheum-test-1")
    .memo("Market from MetaMask")
    .add_message("type.googleapis.com/market.v1.MsgCreateMarketRequest", marketMsg)
    .sign();
```

### Browser (Phantom)

```ts
const signedTx = await TxBuilderWasm.newPhantom()
    .chain_id("morpheum-test-1")
    .memo("Market from Phantom")
    .add_message("type.googleapis.com/market.v1.MsgCreateMarketRequest", marketMsg)
    .sign();
```

---

## 📖 Examples

- `examples/native.rs` — Native Morpheum signer (recommended for most humans)
- `examples/native_agent.rs` — AI Agent with TradingKey + VC claim
- `examples/browser_metamask.ts` — Browser MetaMask / EVM
- `examples/browser_phantom.ts` — Browser Phantom / Solana

Run them with:

```bash
cargo run --example native
cargo run --example native_agent
```

---

## 🛠 Useful Commands

```bash
# Check everything
cargo check

# Run all tests
cargo test --test integration

# Run examples
cargo run --example native
cargo run --example native_agent

# Build WASM for browser
cd crates/wasm
wasm-pack build --target web --release
```

---

## Supported Wallets & Chains

| Signer / Wallet     | Chain      | Type                  | Recommended Use Case       |
|---------------------|------------|-----------------------|----------------------------|
| `NativeSigner`      | Morpheum   | Local ed25519         | Humans with native keys    |
| `AgentSigner`       | Morpheum   | TradingKey + VC       | Autonomous AI agents       |
| MetaMaskAdapter     | EVM        | Injected              | Browser dApps              |
| PhantomAdapter      | Solana     | Injected              | Browser Solana dApps       |
| TaprootAdapter      | Bitcoin    | Injected (Unisat etc.)| Taproot / Bitcoin flows    |
| EvmSigner           | EVM        | Local secp256k1       | Headless EVM signing       |
| SolanaSigner        | Solana     | Local ed25519         | Headless Solana signing    |
| BitcoinSigner       | Bitcoin    | Local BIP-340         | Headless Taproot signing   |

---

## Architecture

- `core/` — `no_std` core (traits, types, generic `TxBuilder`)
- `native/` — Concrete signers, adapters, and nonce providers for Rust
- `wasm/` — Browser + TypeScript bindings with clean factory methods

---

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

**Made with ❤️ for the Morpheum ecosystem**

Questions? Open an issue or reach out on X.
