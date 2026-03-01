# morpheum-signing

[![Crates.io](https://img.shields.io/crates/v/morpheum-signing.svg)](https://crates.io/crates/morpheum-signing)
[![docs.rs](https://img.shields.io/docsrs/morpheum-signing)](https://docs.rs/morpheum-signing)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)

**Universal multi-chain signing SDK for Morpheum** — the official library for humans and AI agents.

Sign transactions from **MetaMask, Phantom, Taproot, native keys**, and **TradingKey + VC delegation** for autonomous agents — all with a single, elegant fluent API.

---

## Features

- **Unified API** — One `TxBuilder` for all signing scenarios
- **Multi-chain support** — Native (ed25519), EVM/secp256k1 (MetaMask), Solana/ed25519 (Phantom), Bitcoin/BIP-340 Schnorr (Taproot)
- **Dynamic `SignerInfo`** — Each signer produces the correct `public_key` protobuf type and `SignMode` automatically
- **Claim embedding** — `TradingKeyClaim` is embedded in `SignerInfo.signing_options` and covered by the transaction signature
- **Claim verification** — `verify()` checks issuer identity; `claim_digest()` for offline cryptographic verification
- **BIP-39 mnemonic** — `NativeSigner::from_mnemonic()` for human-friendly key derivation
- **Agent-first** — `AgentSigner` with `TradingKeyClaim` and isolated nonce sub-ranges for unlimited parallelism
- **Dual target** — Native Rust (CLI, bots, agents) + WASM/TypeScript (browser frontends)
- **Proto-centric** — Produces exact Morpheum `Tx`, `SignDoc`, `TxRaw`, `Nonce`
- **Zero-copy & secure** — `ZeroizeOnDrop` on all secret material, `no_std` core, constant-time cryptographic operations
- **Fuzz tested** — `cargo-fuzz` targets for seed generation, claim construction, address mapping, and claim encoding

---

## Installation

**Rust (native / CLI / bots / agents)**

```toml
# Cargo.toml
[dependencies]
morpheum-signing-native = { version = "0.1", features = ["full"] }
```

**Browser / TypeScript (React, Vue, Svelte, Next.js, etc.)**

```bash
npm install @morpheum/signing
```

---

## Quick Start

### Native (Recommended for most humans)

```rust
use morpheum_signing_native::prelude::*;

let signer = NativeSigner::from_seed(&[42u8; 32]);

let signed_tx = native(signer)
    .chain_id("morpheum-test-1")
    .memo("Test market from native signer")
    .add_message(market_any)        // prost_types::Any — fully generic
    .sign()
    .await?;
```

### Native from BIP-39 mnemonic

```rust
use morpheum_signing_native::prelude::*;

let signer = NativeSigner::from_mnemonic(
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    "",  // passphrase (empty = default)
)?;

let signed_tx = native(signer)
    .chain_id("morpheum-1")
    .add_message(market_any)
    .sign()
    .await?;
```

### AI Agent (TradingKey + VC delegation)

```rust
use morpheum_signing_native::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

let now_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

let claim = VcClaimBuilder::new()
    .issuer(agent_id.clone())
    .subject(agent_id.clone())
    .permissions(1 << 0)                   // TRADE
    .nonce_sub_range(1000, 2000)
    .expiry(now_secs + 86_400)             // 24 hours
    .signature(Signature::Ed25519(sig))    // real issuer signature
    .build(now_secs)?;

let signed_tx = agent(signer)
    .with_trading_key_claim(claim)
    .add_message(market_any)
    .sign()
    .await?;
```

### Agent with claim verification

```rust
// Verify claim before using it
claim.verify(now_secs, &issuer_pubkey)?;

// The claim is embedded in SignerInfo.signing_options
// and covered by the transaction signature.
let signed_tx = agent(signer)
    .with_trading_key_claim(claim)
    .add_message(market_any)
    .sign()
    .await?;
```

### EVM (local secp256k1)

```rust
let signer = EvmSigner::from_seed(&[42u8; 32]);
let signed_tx = evm(signer)
    .chain_id("morpheum-1")
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

### Browser (Taproot)

```ts
const signedTx = await TxBuilderWasm.newTaproot()
    .chain_id("morpheum-test-1")
    .memo("Market from Taproot")
    .add_message("type.googleapis.com/market.v1.MsgCreateMarketRequest", marketMsg)
    .sign();
```

---

## Examples

| Example | Location | Description |
|---------|----------|-------------|
| `native` | `crates/native/examples/native.rs` | Native Morpheum signer (ed25519) with mnemonic support |
| `agent` | `crates/native/examples/agent.rs` | AI Agent with TradingKey + VC claim |
| `agent_with_claim_verification` | `crates/native/examples/agent_with_claim_verification.rs` | Agent with full claim verification before signing |
| Browser MetaMask | `examples/browser_metamask.ts` | Browser MetaMask / EVM with claim support |
| Browser Phantom | `examples/browser_phantom.ts` | Browser Phantom / Solana |

Run Rust examples with:

```bash
cargo run -p morpheum-signing-native --example native
cargo run -p morpheum-signing-native --example agent
cargo run -p morpheum-signing-native --example agent_with_claim_verification
```

---

## Supported Signers & Wallets

| Signer / Wallet | Chain | Curve / Type | `PublicKey` Variant | `SignMode` | Recommended For |
|---|---|---|---|---|---|
| `NativeSigner` | Morpheum | ed25519 | `Ed25519` | `Ed25519` | Humans with native keys |
| `AgentSigner` | Morpheum | ed25519 (TradingKey) | `Agent` | `Ed25519` | Autonomous AI agents |
| `EvmSigner` | EVM | secp256k1 | `Secp256k1` | `Secp256k1` | Headless EVM signing |
| `SolanaSigner` | Solana | ed25519 | `Ed25519` | `Ed25519` | Headless Solana signing |
| `BitcoinSigner` | Bitcoin | BIP-340 Schnorr | `Schnorr` | `SchnorrAggregate` | Headless Taproot signing |
| `MetaMaskAdapter` | EVM | Injected | `Secp256k1` | `Secp256k1` | Browser EVM dApps |
| `PhantomAdapter` | Solana | Injected | `Ed25519` | `Ed25519` | Browser Solana dApps |
| `TaprootAdapter` | Bitcoin | Injected | `Schnorr` | `SchnorrAggregate` | Unisat / Leather / Xverse |

All signers implement the core `Signer` trait. The `TxBuilder` automatically produces
the correct `SignerInfo.public_key` protobuf encoding and `ModeInfo.sign_mode` via the
`public_key_proto()` and `sign_mode()` trait methods.

---

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `full` | Yes | Enables everything below |
| `full-crypto` | via `full` | All crypto backends (ed25519, secp256k1, schnorr) |
| `bip39` | via `full` | BIP-39 mnemonic key derivation |
| `claim-verification` | via `full` | `TradingKeyClaim::verify()` method |
| `dynamic-signer-info` | via `full` | Per-signer `public_key_proto()` and `sign_mode()` |
| `http` | via `full` | Nonce providers (Sentry + AgentPortal) |
| `evm` | via `full-crypto` | `EvmSigner` (secp256k1) |
| `solana` | via `full-crypto` | `SolanaSigner` (ed25519) |
| `bitcoin` | via `full-crypto` | `BitcoinSigner` (BIP-340 Schnorr) |

---

## Architecture

```
morpheum-signing/
├── crates/
│   ├── core/       no_std core: Signer trait, TxBuilder, types, claim, error
│   ├── native/     std: NativeSigner, AgentSigner, EvmSigner, SolanaSigner, BitcoinSigner
│   │   └── examples/  Rust examples (native, agent, agent_with_claim_verification)
│   └── wasm/       WASM + TS: TxBuilderWasm, MetaMask/Phantom/Taproot adapters
├── examples/       Browser/TypeScript examples (MetaMask, Phantom)
├── fuzz/           cargo-fuzz targets for security testing
├── SECURITY.md     Security checklist and vulnerability reporting
└── README.md       This file
```

- **`core/`** — `no_std` core (traits, types, generic `TxBuilder`, claim handling, error types)
- **`native/`** — Concrete local signers, nonce providers, and multi-chain support
- **`wasm/`** — Browser + TypeScript bindings with factory methods (`newMetamask()`, `newPhantom()`, `newTaproot()`)

---

## Production Readiness

This SDK is designed for production deployment with the following guarantees:

**Security**
- All secret key material is protected with `ZeroizeOnDrop` — keys are zeroed when dropped.
- Signing operations use constant-time cryptographic libraries (`ed25519-dalek`, `k256`, `libsecp256k1`).
- No secrets in logs, `Debug` output, or panic messages.
- See [`SECURITY.md`](SECURITY.md) for the full security checklist.

**Correctness**
- Dynamic `SignerInfo` generation — each signer produces the correct protobuf `public_key` and `SignMode`.
- `TradingKeyClaim` is validated (expiry, nonce range, signature) and embedded in the signed transaction.
- Deterministic claim digest via prost protobuf encoding + SHA-256.
- 98+ integration tests covering all signers, claim flows, error paths, and edge cases.
- Fuzz targets for seed generation, claim construction, address mapping, and encoding.

**Robustness**
- Comprehensive `SigningError` enum with clear, actionable messages.
- Input validation: empty transactions are rejected, claims are validated before embedding.
- Multi-chain address mapping handles all address formats deterministically.
- Feature-gated capabilities for minimal attack surface in constrained builds.

**Compatibility**
- `no_std` core for embedded and WASM use.
- Full WASM + TypeScript support with rich type definitions.
- Backward-compatible: new features are opt-in via feature flags.

---

## Useful Commands

```bash
# Check everything compiles
cargo check --all-features

# Run all tests (98+ tests)
cargo test --all-features

# Run clippy (zero warnings)
cargo clippy --all-features --all-targets

# Run examples
cargo run -p morpheum-signing-native --example native
cargo run -p morpheum-signing-native --example agent
cargo run -p morpheum-signing-native --example agent_with_claim_verification

# Build WASM for browser
cd crates/wasm
wasm-pack build crates/wasm --target web --release

# Run fuzz targets (requires nightly + cargo-fuzz)
cd fuzz
cargo +nightly fuzz run fuzz_seed_generation
cargo +nightly fuzz run fuzz_claim_construction
cargo +nightly fuzz run fuzz_address_mapping
cargo +nightly fuzz run fuzz_claim_encoding
```

---

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

**Made with care for the Morpheum ecosystem.**

Questions? Open an issue or reach out on X.
