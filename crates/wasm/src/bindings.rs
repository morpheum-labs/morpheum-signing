//! WASM Bindings — All JavaScript/TypeScript interop for the browser.
//!
//! This file contains **all** `#[wasm_bindgen]` exports and rich TypeScript definitions.
//! The signing crate remains **completely generic** — no knowledge of any specific module messages.
//! Messages are added exclusively as raw `prost_types::Any` (type_url + bytes) from JavaScript/TS.
//!
//! **Architecture**:
//! - Factory methods (`newMetamask`, `newPhantom`, `newTaproot`) are **async** — they
//!   connect to the injected wallet and cache the public key / address before returning.
//! - `TxBuilderWasm` uses the [`WasmSigner`] enum for zero-cost static dispatch.
//! - Full `TradingKeyClaim` support for agent delegation flows.
//! - Excellent TypeScript DX with comprehensive type definitions and JSDoc.
//! - Production-ready: robust error handling, zero-copy where possible, clear messages.

use prost::Message;
use serde::Deserialize;
use sha2::Digest;
use wasm_bindgen::prelude::*;

use crate::adapters::{
    MetaMaskAdapterWasm, PhantomAdapterWasm, TaprootAdapterWasm, WasmSigner,
};
use crate::core::{
    builder::TxBuilder as CoreTxBuilder,
    claim::{TradingKeyClaim, VcClaimBuilder},
    prelude::*,
};

// ==================== MAIN TX BUILDER FOR BROWSER ====================

/// WASM-friendly transaction builder for browser frontends (React, Vue, Svelte, Next.js, etc.).
///
/// **Completely generic** — messages are added via `add_message(type_url, value)`.
/// Use the async factory methods (`newMetamask()`, `newPhantom()`, `newTaproot()`)
/// which connect to the injected wallet and cache the public key before returning.
#[wasm_bindgen]
pub struct TxBuilderWasm {
    inner: CoreTxBuilder<WasmSigner>,
}

#[wasm_bindgen]
impl TxBuilderWasm {
    // ==================== ASYNC FACTORY METHODS ====================

    /// Creates a builder backed by **MetaMask** (or any EVM injected wallet).
    ///
    /// Connects to `window.ethereum`, requests account access, and caches the
    /// EVM address. Returns an error if MetaMask is not available or the user
    /// rejects the connection.
    #[wasm_bindgen(js_name = "newMetamask")]
    pub async fn new_metamask() -> Result<TxBuilderWasm, JsValue> {
        let adapter = MetaMaskAdapterWasm::connect()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self {
            inner: CoreTxBuilder::new(WasmSigner::MetaMask(adapter)),
        })
    }

    /// Creates a builder backed by **Phantom** (or any Solana injected wallet).
    ///
    /// Connects to `window.phantom.solana`, requests wallet access, and caches
    /// the ed25519 public key (32 bytes).
    #[wasm_bindgen(js_name = "newPhantom")]
    pub async fn new_phantom() -> Result<TxBuilderWasm, JsValue> {
        let adapter = PhantomAdapterWasm::connect()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self {
            inner: CoreTxBuilder::new(WasmSigner::Phantom(adapter)),
        })
    }

    /// Creates a builder backed by **Unisat / Leather / Xverse** (Bitcoin Taproot).
    ///
    /// Connects to `window.unisat`, requests account access, and caches
    /// the BIP-340 x-only Schnorr public key (32 bytes).
    #[wasm_bindgen(js_name = "newTaproot")]
    pub async fn new_taproot() -> Result<TxBuilderWasm, JsValue> {
        let adapter = TaprootAdapterWasm::connect()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self {
            inner: CoreTxBuilder::new(WasmSigner::Taproot(adapter)),
        })
    }

    // ==================== BUILDER METHODS (fluent chaining) ====================

    /// Sets the chain ID.
    #[wasm_bindgen(js_name = "chainId")]
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

    /// Sets the account number.
    #[wasm_bindgen(js_name = "accountNumber")]
    pub fn account_number(mut self, account_number: u64) -> TxBuilderWasm {
        self.inner = self.inner.account_number(account_number);
        self
    }

    /// Sets timeout in seconds since epoch.
    #[wasm_bindgen(js_name = "timeoutSeconds")]
    pub fn timeout_seconds(mut self, seconds: u64) -> TxBuilderWasm {
        self.inner = self.inner.timeout_seconds(seconds);
        self
    }

    /// **Generic message adder** — the only way to add messages.
    /// Pass the protobuf type URL and encoded bytes as a `Uint8Array`.
    #[wasm_bindgen(js_name = "addMessage")]
    pub fn add_message(mut self, type_url: String, value: Vec<u8>) -> TxBuilderWasm {
        let any = prost_types::Any { type_url, value };
        self.inner = self.inner.add_message(any);
        self
    }

    // ==================== TRADING KEY CLAIM (Agent Delegation) ====================

    /// Attaches a `TradingKeyClaim` for agent delegation.
    ///
    /// The claim object should have the following fields:
    /// - `issuer`: `Uint8Array(32)` — issuer AccountId
    /// - `subject`: `Uint8Array(32)` — subject AccountId
    /// - `permissions`: `number` — permission bitflags
    /// - `max_daily_usd`: `number` — daily USD spending limit
    /// - `expiry_timestamp`: `number` — Unix seconds expiry
    /// - `nonce_sub_range_start`: `number` — sub-range start (inclusive)
    /// - `nonce_sub_range_end`: `number` — sub-range end (exclusive)
    /// - `signature`: `Uint8Array(64)` — issuer's signature
    /// - `signature_type`: `string` — "ed25519", "secp256k1", or "schnorr"
    #[wasm_bindgen(js_name = "withClaim")]
    pub fn with_claim(mut self, claim_js: JsValue) -> Result<TxBuilderWasm, JsValue> {
        let js_claim: TradingKeyClaimJs = serde_wasm_bindgen::from_value(claim_js)
            .map_err(|e| JsValue::from_str(&format!("Invalid claim object: {e}")))?;

        let claim = js_claim
            .into_claim()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        self.inner = self.inner.with_trading_key_claim(claim);
        Ok(self)
    }

    // ==================== FINAL SIGNING ====================

    /// Final signing call — returns a Promise that resolves to `SignedTx`.
    ///
    /// This builds the transaction, embeds the claim (if present), signs it
    /// with the connected wallet, and returns the fully signed transaction.
    ///
    /// The result is a JS object with:
    /// - `raw_bytes: Uint8Array` — ready for broadcast
    /// - `tx_raw_bytes: Uint8Array` — TxRaw protobuf bytes (if available)
    /// - `txhash: string` — SHA-256 hex of raw_bytes
    #[wasm_bindgen]
    pub async fn sign(self) -> Result<JsValue, JsValue> {
        let signed_tx = self
            .inner
            .sign()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Build a JS-friendly result object
        let obj = js_sys::Object::new();

        // raw_bytes — the canonical broadcast payload
        let raw_bytes = js_sys::Uint8Array::from(signed_tx.raw_bytes());
        js_sys::Reflect::set(&obj, &"raw_bytes".into(), &raw_bytes)
            .map_err(|_| JsValue::from_str("failed to set raw_bytes"))?;

        // tx_raw_bytes — optional TxRaw protobuf bytes
        if let Some(tx_raw) = signed_tx.tx_raw() {
            let tx_raw_bytes = js_sys::Uint8Array::from(tx_raw.encode_to_vec().as_slice());
            js_sys::Reflect::set(&obj, &"tx_raw_bytes".into(), &tx_raw_bytes)
                .map_err(|_| JsValue::from_str("failed to set tx_raw_bytes"))?;
        }

        // tx_bytes — full Tx protobuf bytes (for debugging/inspection)
        let tx_bytes = js_sys::Uint8Array::from(signed_tx.tx().encode_to_vec().as_slice());
        js_sys::Reflect::set(&obj, &"tx_bytes".into(), &tx_bytes)
            .map_err(|_| JsValue::from_str("failed to set tx_bytes"))?;

        // txhash — SHA-256 hex of raw_bytes
        let hash = sha2::Sha256::digest(signed_tx.raw_bytes());
        let txhash = hex::encode(hash);
        js_sys::Reflect::set(&obj, &"txhash".into(), &JsValue::from_str(&txhash))
            .map_err(|_| JsValue::from_str("failed to set txhash"))?;

        Ok(obj.into())
    }
}

// ==================== JS-FRIENDLY CLAIM DESERIALIZATION ====================

/// Internal JS-friendly representation for deserializing `TradingKeyClaim` from JS.
#[derive(Deserialize)]
struct TradingKeyClaimJs {
    issuer: Vec<u8>,
    subject: Vec<u8>,
    permissions: u64,
    max_daily_usd: u64,
    expiry_timestamp: u64,
    nonce_sub_range_start: u32,
    nonce_sub_range_end: u32,
    signature: Vec<u8>,
    signature_type: String,
}

impl TradingKeyClaimJs {
    /// Converts the JS-friendly representation into the core `TradingKeyClaim`.
    fn into_claim(self) -> Result<TradingKeyClaim, morpheum_signing_core::SigningError> {
        use morpheum_signing_core::SigningError;

        let issuer_arr: [u8; 32] = self
            .issuer
            .try_into()
            .map_err(|_| SigningError::invalid_claim("issuer must be exactly 32 bytes"))?;

        let subject_arr: [u8; 32] = self
            .subject
            .try_into()
            .map_err(|_| SigningError::invalid_claim("subject must be exactly 32 bytes"))?;

        let sig_arr: [u8; 64] = self
            .signature
            .try_into()
            .map_err(|_| SigningError::invalid_claim("signature must be exactly 64 bytes"))?;

        let signature = match self.signature_type.as_str() {
            "ed25519" => Signature::Ed25519(sig_arr),
            "secp256k1" => Signature::Secp256k1(sig_arr),
            "schnorr" => Signature::Schnorr(sig_arr),
            other => {
                return Err(SigningError::invalid_claim(format!(
                    "unknown signature_type: '{other}' (expected 'ed25519', 'secp256k1', or 'schnorr')"
                )));
            }
        };

        Ok(TradingKeyClaim {
            issuer: AccountId(issuer_arr),
            subject: AccountId(subject_arr),
            permissions: self.permissions,
            max_daily_usd: self.max_daily_usd,
            expiry_timestamp: self.expiry_timestamp,
            nonce_sub_range_start: self.nonce_sub_range_start,
            nonce_sub_range_end: self.nonce_sub_range_end,
            signature,
        })
    }
}

// ==================== VC CLAIM BUILDER (Standalone) ====================

/// Standalone wasm_bindgen wrapper for building a `TradingKeyClaim` from JS.
///
/// Usage from TypeScript:
/// ```typescript
/// const claim = new VcClaimBuilderWasm()
///     .issuer(issuerBytes)
///     .subject(subjectBytes)
///     .permissions(0x01)
///     .maxDailyUsd(10000)
///     .expiry(Math.floor(Date.now() / 1000) + 86400)
///     .nonceSubRange(100, 200)
///     .signature(sigBytes, "ed25519")
///     .build(Math.floor(Date.now() / 1000));
/// ```
#[wasm_bindgen(js_name = "VcClaimBuilder")]
pub struct VcClaimBuilderWasm {
    inner: VcClaimBuilder,
    signature_type: Option<String>,
}

#[wasm_bindgen(js_class = "VcClaimBuilder")]
impl VcClaimBuilderWasm {
    /// Creates a new empty builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: VcClaimBuilder::new(),
            signature_type: None,
        }
    }

    /// Sets the issuer AccountId (32 bytes).
    #[wasm_bindgen]
    pub fn issuer(mut self, bytes: Vec<u8>) -> Result<VcClaimBuilderWasm, JsValue> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| JsValue::from_str("issuer must be exactly 32 bytes"))?;
        self.inner = self.inner.issuer(AccountId(arr));
        Ok(self)
    }

    /// Sets the subject AccountId (32 bytes).
    #[wasm_bindgen]
    pub fn subject(mut self, bytes: Vec<u8>) -> Result<VcClaimBuilderWasm, JsValue> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| JsValue::from_str("subject must be exactly 32 bytes"))?;
        self.inner = self.inner.subject(AccountId(arr));
        Ok(self)
    }

    /// Sets the permission bitflags.
    #[wasm_bindgen]
    pub fn permissions(mut self, perms: u64) -> VcClaimBuilderWasm {
        self.inner = self.inner.permissions(perms);
        self
    }

    /// Sets the daily USD spending limit.
    #[wasm_bindgen(js_name = "maxDailyUsd")]
    pub fn max_daily_usd(mut self, amount: u64) -> VcClaimBuilderWasm {
        self.inner = self.inner.max_daily_usd(amount);
        self
    }

    /// Sets the expiry timestamp (Unix seconds).
    #[wasm_bindgen]
    pub fn expiry(mut self, timestamp: u64) -> VcClaimBuilderWasm {
        self.inner = self.inner.expiry(timestamp);
        self
    }

    /// Sets the nonce sub-range [start, end).
    #[wasm_bindgen(js_name = "nonceSubRange")]
    pub fn nonce_sub_range(mut self, start: u32, end: u32) -> VcClaimBuilderWasm {
        self.inner = self.inner.nonce_sub_range(start, end);
        self
    }

    /// Sets the issuer's signature (64 bytes) and signature type.
    #[wasm_bindgen]
    pub fn signature(
        mut self,
        sig_bytes: Vec<u8>,
        sig_type: String,
    ) -> Result<VcClaimBuilderWasm, JsValue> {
        let arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| JsValue::from_str("signature must be exactly 64 bytes"))?;
        let sig = match sig_type.as_str() {
            "ed25519" => Signature::Ed25519(arr),
            "secp256k1" => Signature::Secp256k1(arr),
            "schnorr" => Signature::Schnorr(arr),
            _ => return Err(JsValue::from_str("signature_type must be 'ed25519', 'secp256k1', or 'schnorr'")),
        };
        self.inner = self.inner.signature(sig);
        self.signature_type = Some(sig_type);
        Ok(self)
    }

    /// Builds and validates the claim. `current_timestamp` is Unix seconds.
    #[wasm_bindgen]
    pub fn build(self, current_timestamp: u64) -> Result<JsValue, JsValue> {
        let claim = self
            .inner
            .build(current_timestamp)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Return as a JS-friendly object
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"issuer".into(), &js_sys::Uint8Array::from(&claim.issuer.0[..]).into())
            .map_err(|_| JsValue::from_str("failed to set issuer"))?;
        js_sys::Reflect::set(&obj, &"subject".into(), &js_sys::Uint8Array::from(&claim.subject.0[..]).into())
            .map_err(|_| JsValue::from_str("failed to set subject"))?;
        js_sys::Reflect::set(&obj, &"permissions".into(), &JsValue::from(claim.permissions as f64))
            .map_err(|_| JsValue::from_str("failed to set permissions"))?;
        js_sys::Reflect::set(&obj, &"max_daily_usd".into(), &JsValue::from(claim.max_daily_usd as f64))
            .map_err(|_| JsValue::from_str("failed to set max_daily_usd"))?;
        js_sys::Reflect::set(&obj, &"expiry_timestamp".into(), &JsValue::from(claim.expiry_timestamp as f64))
            .map_err(|_| JsValue::from_str("failed to set expiry_timestamp"))?;
        js_sys::Reflect::set(&obj, &"nonce_sub_range_start".into(), &JsValue::from(claim.nonce_sub_range_start))
            .map_err(|_| JsValue::from_str("failed to set nonce_sub_range_start"))?;
        js_sys::Reflect::set(&obj, &"nonce_sub_range_end".into(), &JsValue::from(claim.nonce_sub_range_end))
            .map_err(|_| JsValue::from_str("failed to set nonce_sub_range_end"))?;
        js_sys::Reflect::set(&obj, &"signature".into(), &js_sys::Uint8Array::from(&claim.signature.to_bytes()[..]).into())
            .map_err(|_| JsValue::from_str("failed to set signature"))?;
        js_sys::Reflect::set(&obj, &"signature_type".into(), &JsValue::from_str(self.signature_type.as_deref().unwrap_or("ed25519")))
            .map_err(|_| JsValue::from_str("failed to set signature_type"))?;
        // Also include the proto-encoded Any for direct embedding
        let any = claim.to_proto_any();
        js_sys::Reflect::set(&obj, &"proto_any_type_url".into(), &JsValue::from_str(&any.type_url))
            .map_err(|_| JsValue::from_str("failed to set proto_any_type_url"))?;
        js_sys::Reflect::set(&obj, &"proto_any_value".into(), &js_sys::Uint8Array::from(&any.value[..]).into())
            .map_err(|_| JsValue::from_str("failed to set proto_any_value"))?;

        Ok(obj.into())
    }
}

// ==================== CONVENIENCE LOGGING ====================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
