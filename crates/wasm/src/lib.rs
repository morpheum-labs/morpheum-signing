//! WASM entrypoint for the Morpheum Signing SDK.
//!
//! Provides clean, TypeScript-friendly bindings for browser frontends.
//! Supports injected wallets (MetaMask, Phantom, etc.) and agent signing.
//!
//! Build with: `wasm-pack build --target web --release`

#![allow(non_snake_case)]
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use tsify::Tsify;
use serde::Serialize;

use morpheum_signing_core as core;
use core::prelude::*;

// ==================== PANIC HOOK FOR BETTER DEBUGGING ====================

#[wasm_bindgen]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// ==================== VERSION ====================

#[wasm_bindgen]
pub fn version() -> String {
    core::VERSION.to_string()
}

// ==================== MAIN WASM BUILDER ====================

/// WASM-friendly wrapper around `TxBuilder` for browser use.
#[wasm_bindgen]
pub struct TxBuilderWasm {
    inner: core::TxBuilder<core::HumanSigner>, // Default to human; can be extended for agent
}

#[wasm_bindgen]
impl TxBuilderWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let dummy_signer = core::HumanSigner::from_seed(&[0u8; 32]);
        Self {
            inner: core::TxBuilder::human(dummy_signer),
        }
    }

    #[wasm_bindgen]
    pub fn chain_id(mut self, chain_id: String) -> TxBuilderWasm {
        self.inner = self.inner.chain_id(chain_id);
        self
    }

    #[wasm_bindgen]
    pub fn memo(mut self, memo: String) -> TxBuilderWasm {
        self.inner = self.inner.memo(memo);
        self
    }

    /// Example: Create a market (placeholder - in full version this would accept a JS object)
    #[wasm_bindgen]
    pub fn create_market(mut self, name: String) -> TxBuilderWasm {
        log(&format!("Adding CreateMarket: {}", name));
        // In real implementation: self.inner = self.inner.create_market(msg);
        self
    }

    #[wasm_bindgen]
    pub async fn sign(self) -> Result<JsValue, JsValue> {
        match self.inner.sign().await {
            Ok(signed_tx) => {
                let serialized = serde_wasm_bindgen::to_value(&signed_tx)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                Ok(serialized)
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }
}

// ==================== TYPE EXPORTS FOR TYPESCRIPT ====================

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
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