//! Browser wallet adapters — re-exported from `morpheum-signing-wasm-lib`.
//!
//! The concrete adapter implementations and the `WasmSigner` dispatch enum
//! live in the shared `morpheum-signing-wasm-lib` crate so they can be
//! reused by both this crate and `morpheum-sdk-wasm`.

pub use morpheum_signing_wasm_lib::{
    MetaMaskAdapterWasm, PhantomAdapterWasm, TaprootAdapterWasm, WasmSigner,
};
