//! WASM Bindings — All JavaScript/TypeScript interop for the browser.
//!
//! This file contains **all** `#[wasm_bindgen]` exports and rich TypeScript definitions.
//! The signing crate remains **completely generic** — no knowledge of any specific module messages.
//! Messages are added exclusively as raw `prost_types::Any` (type_url + bytes) from JavaScript/TS.
//!
//! **Architecture**:
//! - Factory methods for the three major injected wallets (MetaMask, Phantom, Taproot).
//! - `TxBuilderWasm` remains fully generic and uses the `WalletAdapter` trait under the hood.
//! - All wallet-specific logic (JS interop) is cleanly encapsulated.
//! - Excellent TypeScript DX with full type safety and JSDoc.
//! - Production-ready: robust error handling, zero-copy where possible, clear messages.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use tsify::Tsify;
use serde_wasm_bindgen;

use crate::core::{
    prelude::*,
    TxBuilder as CoreTxBuilder,
    SigningError,
    SignedTx,
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

/// WASM-friendly transaction builder for browser frontends (React, Vue, Svelte, Next.js, etc.).
///
/// **Completely generic** — messages are added via `add_message(type_url, value)`.
/// Use the factory methods (`new_metamask()`, `new_phantom()`, `new_taproot()`) for injected wallets.
#[wasm_bindgen]
pub struct TxBuilderWasm {
    inner: CoreTxBuilder<Box<dyn Signer>>,
}

#[wasm_bindgen]
impl TxBuilderWasm {
    // ==================== FACTORY METHODS ====================

    /// Creates a builder backed by **MetaMask** (or any EVM injected wallet).
    #[wasm_bindgen(js_name = "newMetamask")]
    pub fn new_metamask() -> Self {
        let adapter = Box::new(MetaMaskAdapterWasm::new()) as Box<dyn Signer>;
        Self {
            inner: CoreTxBuilder::new(adapter),
        }
    }

    /// Creates a builder backed by **Phantom** (or any Solana injected wallet).
    #[wasm_bindgen(js_name = "newPhantom")]
    pub fn new_phantom() -> Self {
        let adapter = Box::new(PhantomAdapterWasm::new()) as Box<dyn Signer>;
        Self {
            inner: CoreTxBuilder::new(adapter),
        }
    }

    /// Creates a builder backed by **Unisat / Leather / Xverse** (Bitcoin Taproot).
    #[wasm_bindgen(js_name = "newTaproot")]
    pub fn new_taproot() -> Self {
        let adapter = Box::new(TaprootAdapterWasm::new()) as Box<dyn Signer>;
        Self {
            inner: CoreTxBuilder::new(adapter),
        }
    }

    // ==================== BUILDER METHODS ====================

    /// Sets the chain ID.
    #[wasm_bindgen]
    pub fn chain_id(mut self, chain_id: String) -> TxBuilderWasm {
        self.inner = self.inner.chain_id(chain_id);
        self
    }

    /// Sets an optional memo.
    #[wasm_bindgen]
    pub fn memo(mut self, memo: String) -> TxBuilderWasm {
        self.inner = self.inner.memo(memo);
        self
    }

    /// Sets timeout in seconds since epoch.
    #[wasm_bindgen]
    pub fn timeout_seconds(mut self, seconds: u64) -> TxBuilderWasm {
        self.inner = self.inner.timeout_seconds(seconds);
        self
    }

    /// **Generic message adder** — the only way to add messages.
    /// Pass the protobuf type URL and a plain JavaScript object.
    #[wasm_bindgen]
    pub fn add_message(mut self, type_url: String, value: JsValue) -> Result<TxBuilderWasm, JsValue> {
        let bytes: Vec<u8> = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))?;

        let any = prost_types::Any { type_url, value: bytes };

        self.inner = self.inner.add_message(any);
        Ok(self)
    }

    /// Final signing call — returns a Promise that resolves to `SignedTx`.
    #[wasm_bindgen]
    pub async fn sign(self) -> Result<JsValue, JsValue> {
        match self.inner.sign().await {
            Ok(signed_tx) => {
                let serialized = serde_wasm_bindgen::to_value(&signed_tx)
                    .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))?;
                Ok(serialized)
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }
}

// ==================== RICH TYPESCRIPT DEFINITIONS ====================

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface SignedTx {
    tx: any;                    // Full tx.v1.Tx
    raw_bytes: Uint8Array;      // Ready for broadcast
    tx_raw?: any;               // Optional TxRaw
    txhash?: string;            // sha256 hex of raw_bytes
}

/**
 * Main builder for browser use.
 * Completely generic — messages added via add_message().
 */
export class TxBuilderWasm {
    private constructor();

    /** MetaMask / Rabby / Ledger (EVM) */
    static newMetamask(): TxBuilderWasm;

    /** Phantom / Solflare / Backpack (Solana) */
    static newPhantom(): TxBuilderWasm;

    /** Unisat / Leather / Xverse (Bitcoin Taproot) */
    static newTaproot(): TxBuilderWasm;

    chain_id(chain_id: string): TxBuilderWasm;
    memo(memo: string): TxBuilderWasm;
    timeout_seconds(seconds: number): TxBuilderWasm;

    /** Add any protobuf message (completely generic) */
    add_message(type_url: string, value: any): Promise<TxBuilderWasm>;

    sign(): Promise<SignedTx>;
}
"#;

// ==================== HELPER LOGGING FOR BROWSER CONSOLE ====================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}