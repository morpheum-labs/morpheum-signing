//! Injected wallet adapters for browser environments.
//!
//! This module provides concrete implementations of the `WalletAdapter` trait
//! from `morpheum-signing-core` for popular injected wallets:
//!
//! - **MetaMaskAdapter** — EVM / MetaMask (and other EVM wallets like Rabby, Ledger)
//! - **PhantomAdapter** — Solana / Phantom (and Solflare, Backpack, etc.)
//! - **TaprootAdapter** — Bitcoin Taproot / Unisat, Leather, Xverse, etc.
//!
//! These adapters are **browser-only** (WASM target) and use `js_sys` / `wasm-bindgen`
//! to communicate with the injected JavaScript APIs (`window.ethereum`, `window.phantom`, etc.).
//!
//! They are deliberately separate from the local keypair signers in the `signers/` module.
//! This separation follows the **Adapter Pattern** (GoF) and maintains clean boundaries:
//! - `signers/` = you own the private key (CLI, bots, agents)
//! - `adapters/` = external wallet controls the key (browser dApps)
//!
//! Usage in `TxBuilder`:
//! ```rust,ignore
//! let adapter = MetaMaskAdapter::new();
//! let tx = TxBuilder::new(adapter) // or with_wallet_adapter(adapter)
//!     .chain_id("morpheum-test-1")
//!     .add_message(...)
//!     .sign()
//!     .await?;
//! ```

// Module declarations
pub mod metamask;
pub mod phantom;
pub mod taproot;

// Public re-exports for ergonomic use
pub use metamask::MetaMaskAdapter;
pub use phantom::PhantomAdapter;
pub use taproot::TaprootAdapter;

// Convenience type aliases (matching the exact pattern used in `signers/mod.rs`)
// This provides the shortest, most idiomatic names for common usage.
pub type MetaMask = MetaMaskAdapter;
pub type Phantom = PhantomAdapter;
pub type Taproot = TaprootAdapter;

/// Re-export the core trait so users can import everything from one place
/// if they want to implement their own custom adapter.
pub use morpheum_signing_core::wallet_adapter::WalletAdapter;