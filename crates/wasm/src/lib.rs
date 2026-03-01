//! WASM entrypoint for the Morpheum Signing SDK.
//!
//! Provides clean, TypeScript-friendly bindings for browser frontends.
//! Supports injected wallets (MetaMask, Phantom, etc.) via the `WalletAdapter` trait.
//!
//! Build with: `wasm-pack build --target web --release`

#![cfg(target_arch = "wasm32")]
#![allow(non_snake_case)]

use wasm_bindgen::prelude::*;

use morpheum_signing_core as core;

mod bindings;

// ==================== PANIC HOOK FOR BETTER DEBUGGING ====================

/// Installs a panic hook for better browser console error messages.
#[wasm_bindgen]
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// ==================== VERSION ====================

/// Returns the SDK version string.
#[wasm_bindgen]
pub fn version() -> String {
    core::VERSION.to_string()
}

// ==================== TYPE EXPORTS FOR TYPESCRIPT ====================

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface SignedTx {
    tx: any;
    raw_bytes: Uint8Array;
    tx_raw?: any;
}

export interface SigningOptions {
    deadline_seconds?: number;
    memo?: string;
    include_timestamp: boolean;
}
"#;

// ==================== CONVENIENCE LOGGING ====================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
