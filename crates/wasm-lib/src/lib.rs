//! Shared WASM wallet adapters for the Morpheum signing ecosystem.
//!
//! This crate extracts the browser wallet adapter implementations (MetaMask,
//! Phantom, Taproot) and the `WasmSigner` dispatch enum into a reusable
//! `rlib`. Both `morpheum-signing-wasm` (cdylib) and `morpheum-sdk-wasm`
//! (cdylib) can depend on this crate to share wallet integration code without
//! duplicating it.
//!
//! **This crate is NOT a cdylib** — it does not produce a standalone `.wasm`
//! file. It is a library crate consumed by cdylib crates.

#![cfg(target_arch = "wasm32")]
#![allow(non_snake_case)]

pub mod metamask;
pub mod phantom;
mod signer;
pub mod taproot;

pub use metamask::MetaMaskAdapterWasm;
pub use phantom::PhantomAdapterWasm;
pub use signer::WasmSigner;
pub use taproot::TaprootAdapterWasm;
