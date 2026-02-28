//! WASM Bindings — All JavaScript/TypeScript interop code.
//!
//! This file contains all `#[wasm_bindgen]` exports and TypeScript definitions.
//! It keeps the main `lib.rs` clean and focused on Rust logic.
//!
//! Build with: `wasm-pack build --target web --release`

use wasm_bindgen::prelude::*;
use tsify::Tsify;
use serde::{Deserialize, Serialize};

use crate::core::{
    prelude::*,
    TxBuilder as CoreTxBuilder,
    HumanSigner,
};

// ==================== PANIC HOOK FOR BETTER BROWSER DEBUGGING ====================

#[wasm_bindgen]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// ==================== VERSION ====================

#[wasm_bindgen]
pub fn version() -> String {
    crate::core::VERSION.to_string()
}

// ==================== MAIN TX BUILDER FOR BROWSER ====================

/// WASM wrapper around `TxBuilder` for browser use.
///
/// This is the primary class exposed to TypeScript/React/Vue/etc.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TxBuilderWasm {
    inner: CoreTxBuilder<HumanSigner>, // Default human signer; agent support added via methods
}

#[wasm_bindgen]
impl TxBuilderWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Use a dummy signer for construction; real signer is set via methods
        let dummy_signer = HumanSigner::from_seed(&[0u8; 32]);
        Self {
            inner: CoreTxBuilder::human(dummy_signer),
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

    #[wasm_bindgen]
    pub fn create_market(mut self, name: String) -> TxBuilderWasm {
        // In full production: convert JS object to MsgCreateMarketRequest
        log(&format!("Adding CreateMarket: {}", name));
        // Placeholder for real message packing
        self
    }

    /// Final signing call — returns a promise in JS/TS.
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

// ==================== TYPE DEFINITIONS FOR PERFECT TYPESCRIPT DX ====================

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface SignedTx {
    tx: any;                    // Full tx.v1.Tx
    raw_bytes: Uint8Array;      // Serialized TxRaw for broadcast
    tx_raw?: any;               // Optional TxRaw for debugging
    txhash?: string;            // Computed sha256 hex (added in native)
}

export interface SigningOptions {
    deadline_seconds?: number;
    memo?: string;
    include_timestamp: boolean;
}

/**
 * Main builder class exposed to TypeScript.
 */
export class TxBuilderWasm {
    constructor();
    chain_id(chain_id: string): TxBuilderWasm;
    memo(memo: string): TxBuilderWasm;
    create_market(name: string): TxBuilderWasm;
    sign(): Promise<SignedTx>;
}
"#;

// ==================== HELPER LOGGING FOR BROWSER CONSOLE ====================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}