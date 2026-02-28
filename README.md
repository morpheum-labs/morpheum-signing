# morpheum-signing

[![Crates.io](https://img.shields.io/crates/v/morpheum-signing.svg)](https://crates.io/crates/morpheum-signing)
[![docs.rs](https://img.shields.io/docsrs/morpheum-signing)](https://docs.rs/morpheum-signing)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)

**Universal multi-chain signing SDK for Morpheum** — the official library for humans and AI agents.

Sign transactions from **MetaMask, Phantom, Taproot, Ledger, native keys**, and **TradingKey + VC delegation** for autonomous agents — all with a single fluent API.

---

## ✨ Features

- **Unified API** — One `TxBuilder` for humans and agents
- **Multi-chain support** — MetaMask (EVM), Phantom (Solana), Taproot (Bitcoin), Native, Agent DIDs
- **Agent-first** — TradingKey + VC claims with isolated nonce sub-ranges (unlimited parallelism)
- **Dual target** — Native Rust (CLI, bots, agents) + WASM/TypeScript (browser frontends)
- **Proto-centric** — Produces exact Morpheum `Tx`, `SignDoc`, `TxRaw`, `Nonce`
- **Zero-copy & secure** — `zeroize`, `secrecy`, no_std core
- **Production ready** — Full test suite, examples, excellent error messages

---

## 📦 Installation

**Rust (native / CLI / bots / agents)**

```toml
# Cargo.toml
[dependencies]
morpheum-signing = { version = "0.1", features = ["full"] }
```

**Browser / TypeScript (React, Vue, Svelte, etc.)**

```bash
npm install @morpheum/signing
```

---

## 🚀 Quick Start

### Native Human (MetaMask-style)

```rust
use morpheum_signing_native::prelude::*;

let signer = HumanSigner::from_seed(&[42u8; 32]);

let signed_tx = human(signer)
    .chain_id("morpheum-test-1")
    .memo("Test market from human")
    .create_market("BTC-USD-PERP".to_string())
    .sign()
    .await?;
```

### Native AI Agent (TradingKey + VC)

```rust
let claim = VcClaimBuilder::new()
    .issuer(agent_id.clone())
    .subject(agent_id.clone())
    .permissions(1 << 0) // TRADE
    .nonce_sub_range(1000, 2000)
    .build()?;

let signed_tx = agent(signer)
    .with_trading_key_claim(claim)
    .create_market("ETH-USD-PERP".to_string())
    .sign()
    .await?;
```

### Browser (MetaMask / Phantom)

```ts
import { TxBuilderWasm, set_panic_hook } from '@morpheum/signing';

set_panic_hook();

const tx = await new TxBuilderWasm()
    .chain_id("morpheum-test-1")
    .memo("Market from MetaMask")
    .create_market("BTC-USD-PERP")
    .sign();
```

---

## 📖 Examples

- `examples/native_human.rs` — Human signer (CLI/bots)
- `examples/native_agent.rs` — AI Agent with TradingKey
- `examples/browser_metamask.ts` — Browser MetaMask
- `examples/browser_phantom.ts` — Browser Phantom

Run them with:
```bash
cargo run --example native_human
cargo run --example native_agent
```

---

## 🛠 Useful Commands

```bash
# Check everything
cargo check

# Run all tests
cargo test --test integration

# Run specific test suite
cargo test --test integration human_flow
cargo test --test integration agent_flow
cargo test --test integration multi_chain

# Run examples
cargo run --example native_human
cargo run --example native_agent

# Build WASM for browser
cd crates/wasm
wasm-pack build --target web --release

# Build and watch for development
wasm-pack build --target web --dev --watch
```

---

## Supported Wallets & Chains

| Wallet       | Chain     | Mode                  | Supported |
|--------------|-----------|-----------------------|---------|
| MetaMask     | EVM       | Injected / Local      | Yes     |
| Phantom      | Solana    | Injected              | Yes     |
| Taproot      | Bitcoin   | Schnorr               | Yes     |
| Ledger       | Multi     | Hardware              | Yes     |
| Native       | Morpheum  | Local keypair         | Yes     |
| Agent        | Morpheum  | TradingKey + VC       | Yes     |

---

## Architecture

- `core/` — no_std core (traits, types, builder)
- `native/` — Concrete implementations for Rust CLI/bots/agents
- `wasm/` — Browser + TypeScript bindings

---

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

**Made with ❤️ for the Morpheum ecosystem**

Questions? Open an issue or reach out on X.

---