//! Taproot (Bitcoin) injected wallet adapter for WASM/browser environments.
//!
//! Delegates signing to `window.unisat` (also compatible with Leather, Xverse, and
//! other Taproot wallets that expose the same API surface).
//!
//! Uses BIP-322-simple `signMessage` with a human-readable prefixed SHA-256 digest
//! of the canonical `SignDoc`. Returns `Signature::Schnorr` (BIP-340, 64 bytes).
//!
//! Uses [`RefCell`] for interior mutability of the cached Taproot public key,
//! enabling future account-change handling without `&mut self`.

use std::cell::RefCell;

use js_sys::{Object, Reflect};
use prost::Message;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use morpheum_signing_core::{
    error::SigningError,
    proto::tx::v1::SignDoc,
    types::{AccountId, Address, PublicKey, Signature},
};

// ==================== JS INTEROP ====================

#[wasm_bindgen]
extern "C" {
    /// Opaque handle to the `window.unisat` provider object.
    type Unisat;

    #[wasm_bindgen(method, js_name = "getPublicKey")]
    fn get_public_key(this: &Unisat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "requestAccounts")]
    fn request_accounts(this: &Unisat) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "signMessage")]
    fn sign_message(this: &Unisat, message: &str, options: &JsValue) -> js_sys::Promise;
}

/// Retrieves the injected `window.unisat` provider or returns a clear error.
fn get_unisat() -> Result<Unisat, SigningError> {
    let window = web_sys::window()
        .ok_or_else(|| SigningError::wallet_adapter("no window object (not in a browser?)"))?;
    let val = Reflect::get(&window, &JsValue::from_str("unisat"))
        .map_err(|_| SigningError::wallet_adapter("failed to access window.unisat"))?;
    if val.is_undefined() || val.is_null() {
        return Err(SigningError::wallet_adapter(
            "Unisat not detected (window.unisat is missing)",
        ));
    }
    Ok(val.unchecked_into::<Unisat>())
}

// ==================== ADAPTER ====================

/// Taproot (Bitcoin) injected wallet adapter.
///
/// Created via the async [`connect()`](Self::connect) method, which requests
/// account access and caches the BIP-340 x-only public key (32 bytes).
pub struct TaprootAdapterWasm {
    /// Mutable cached x-only Schnorr public key (32 bytes).
    ///
    /// SAFETY: `RefCell` is `!Sync`, but WASM is single-threaded —
    /// no concurrent access is possible.
    cached_pubkey: RefCell<[u8; 32]>,
    /// Cached Taproot address string (bc1p...).
    cached_address: RefCell<String>,
}

// SAFETY: WASM (wasm32-unknown-unknown) is single-threaded by specification.
unsafe impl Send for TaprootAdapterWasm {}
unsafe impl Sync for TaprootAdapterWasm {}

impl TaprootAdapterWasm {
    /// Connects to the Taproot wallet, requests accounts, and caches the x-only public key.
    pub async fn connect() -> Result<Self, SigningError> {
        let unisat = get_unisat()?;

        // Request account access
        let promise = unisat.request_accounts();
        let result = JsFuture::from(promise)
            .await
            .map_err(|e| SigningError::wallet_adapter(format!("Unisat requestAccounts failed: {e:?}")))?;

        let accounts = js_sys::Array::from(&result);
        let address = accounts
            .get(0)
            .as_string()
            .ok_or_else(|| SigningError::wallet_adapter("Unisat returned no accounts"))?;

        // Fetch the x-only public key (hex string, 64 chars = 32 bytes)
        let pk_promise = unisat.get_public_key();
        let pk_result = JsFuture::from(pk_promise)
            .await
            .map_err(|e| SigningError::wallet_adapter(format!("Unisat getPublicKey failed: {e:?}")))?;

        let pk_hex: String = pk_result
            .as_string()
            .ok_or_else(|| SigningError::wallet_adapter("Unisat getPublicKey returned non-string"))?;

        let pk_bytes = hex::decode(&pk_hex)
            .map_err(|e| SigningError::wallet_adapter(format!("invalid public key hex from Unisat: {e}")))?;

        // Accept both 32-byte (x-only) and 33-byte (compressed) formats
        let mut pubkey = [0u8; 32];
        match pk_bytes.len() {
            32 => pubkey.copy_from_slice(&pk_bytes),
            33 => pubkey.copy_from_slice(&pk_bytes[1..]), // strip the prefix byte
            other => {
                return Err(SigningError::wallet_adapter(format!(
                    "Unisat returned unexpected public key length: {other} (expected 32 or 33)"
                )));
            }
        }

        Ok(Self {
            cached_pubkey: RefCell::new(pubkey),
            cached_address: RefCell::new(address),
        })
    }

    /// Signs the canonical `SignDoc` using BIP-322-simple `signMessage` via Unisat.
    pub(crate) async fn sign_impl(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let unisat = get_unisat()?;

        // Build human-readable prefixed message
        let message = Self::build_sign_message(sign_doc);

        // BIP-322-simple signing options
        let options = Object::new();
        Reflect::set(&options, &JsValue::from_str("type"), &JsValue::from_str("bip322-simple"))
            .map_err(|_| SigningError::wallet_adapter("failed to set signing options"))?;

        let promise = unisat.sign_message(&message, &options);
        let result = JsFuture::from(promise)
            .await
            .map_err(|e| SigningError::wallet_adapter(format!("Unisat signMessage failed: {e:?}")))?;

        // Unisat may return the signature directly or nested in an object
        let sig_str = if let Some(s) = result.as_string() {
            s
        } else {
            let nested = Reflect::get(&result, &JsValue::from_str("signature"))
                .map_err(|_| SigningError::wallet_adapter("Unisat response missing 'signature'"))?;
            nested
                .as_string()
                .ok_or_else(|| SigningError::wallet_adapter("Unisat signature is not a string"))?
        };

        // Decode: hex (0x-prefixed) or base64
        let sig_bytes = if let Some(hex_str) = sig_str.strip_prefix("0x") {
            hex::decode(hex_str)
                .map_err(|e| SigningError::wallet_adapter(format!("invalid hex signature: {e}")))?
        } else {
            // Try hex first (common for Unisat), then fall back to assuming raw hex
            hex::decode(&sig_str).unwrap_or_else(|_| sig_str.as_bytes().to_vec())
        };

        if sig_bytes.len() != 64 {
            return Err(SigningError::wallet_adapter(format!(
                "Unisat returned invalid Schnorr signature length: {} (expected 64)",
                sig_bytes.len()
            )));
        }

        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        Ok(Signature::Schnorr(arr))
    }

    /// Returns the cached BIP-340 x-only Schnorr public key.
    pub(crate) fn public_key(&self) -> PublicKey {
        PublicKey::Schnorr(*self.cached_pubkey.borrow())
    }

    /// Returns the protobuf-encoded public key for `SignerInfo`.
    pub(crate) fn public_key_proto(&self) -> prost_types::Any {
        prost_types::Any {
            type_url: "/morpheum.crypto.schnorr.PubKey".to_string(),
            value: self.cached_pubkey.borrow().to_vec(),
        }
    }

    /// Derives the `AccountId` from the cached Taproot address.
    pub(crate) fn account_id(&self) -> AccountId {
        let addr = self.cached_address.borrow().clone();
        Address::Bitcoin(addr).to_account_id()
    }

    // ==================== PRIVATE HELPERS ====================

    /// Builds a clear, human-readable message for BIP-322-simple `signMessage`.
    ///
    /// Format: `"Morpheum SignDoc v1\n<sha256_hex(SignDoc)>"`
    fn build_sign_message(sign_doc: &SignDoc) -> String {
        let bytes = sign_doc.encode_to_vec();
        let hash = Sha256::digest(bytes);
        format!("Morpheum SignDoc v1\n{}", hex::encode(hash))
    }
}
