//! MetaMask (EVM) injected wallet adapter for WASM/browser environments.
//!
//! Delegates signing to `window.ethereum` via EIP-712 typed data signing
//! (`eth_signTypedData_v4`), the secure and widely supported industry standard.
//!
//! Uses [`RefCell`] for interior mutability of the cached EVM address,
//! enabling future account-change event handling without `&mut self`.

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
    /// Opaque handle to the `window.ethereum` provider object.
    type Ethereum;

    #[wasm_bindgen(method, js_name = "request")]
    fn request(this: &Ethereum, params: &JsValue) -> js_sys::Promise;
}

/// Retrieves the injected `window.ethereum` provider or returns a clear error.
fn get_ethereum() -> Result<Ethereum, SigningError> {
    let window = web_sys::window()
        .ok_or_else(|| SigningError::wallet_adapter("no window object (not in a browser?)"))?;
    let val = Reflect::get(&window, &JsValue::from_str("ethereum"))
        .map_err(|_| SigningError::wallet_adapter("failed to access window.ethereum"))?;
    if val.is_undefined() || val.is_null() {
        return Err(SigningError::wallet_adapter(
            "MetaMask not detected (window.ethereum is missing)",
        ));
    }
    Ok(val.unchecked_into::<Ethereum>())
}

// ==================== ADAPTER ====================

/// MetaMask (EVM) injected wallet adapter.
///
/// Created via the async [`connect()`](Self::connect) method, which requests
/// `eth_requestAccounts` and caches the first returned EVM address.
pub struct MetaMaskAdapterWasm {
    /// Mutable cached EVM address (20 bytes). Updated on reconnection.
    ///
    /// SAFETY: `RefCell` is `!Sync`, but WASM is single-threaded —
    /// no concurrent access is possible.
    cached_address: RefCell<[u8; 20]>,
    /// Hex address string (0x...) for passing back to MetaMask APIs.
    address_hex: String,
}

// SAFETY: WASM (wasm32-unknown-unknown) is single-threaded by specification.
unsafe impl Send for MetaMaskAdapterWasm {}
unsafe impl Sync for MetaMaskAdapterWasm {}

impl MetaMaskAdapterWasm {
    /// Connects to MetaMask, requests account access, and caches the first address.
    ///
    /// This is the only way to create an adapter — ensures the wallet is connected
    /// and the public key data is available before any signing attempt.
    pub async fn connect() -> Result<Self, SigningError> {
        let ethereum = get_ethereum()?;

        // Request account access
        let request_obj = Object::new();
        Reflect::set(
            &request_obj,
            &JsValue::from_str("method"),
            &JsValue::from_str("eth_requestAccounts"),
        )
        .map_err(|_| SigningError::wallet_adapter("failed to build request object"))?;

        let promise = ethereum.request(&request_obj.into());
        let result = JsFuture::from(promise).await.map_err(|e| {
            SigningError::wallet_adapter(format!("eth_requestAccounts failed: {e:?}"))
        })?;

        // Parse first account
        let accounts = js_sys::Array::from(&result);
        let first = accounts
            .get(0)
            .as_string()
            .ok_or_else(|| SigningError::wallet_adapter("MetaMask returned no accounts"))?;

        let hex_str = first.strip_prefix("0x").unwrap_or(&first);
        let mut address_bytes = [0u8; 20];
        hex::decode_to_slice(hex_str, &mut address_bytes).map_err(|e| {
            SigningError::wallet_adapter(format!("invalid EVM address from MetaMask: {e}"))
        })?;

        Ok(Self {
            cached_address: RefCell::new(address_bytes),
            address_hex: first,
        })
    }

    /// Signs the canonical `SignDoc` using EIP-712 typed data via MetaMask.
    pub(crate) async fn sign_impl(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let ethereum = get_ethereum()?;

        // Build EIP-712 payload
        let payload = self.build_eip712_payload(sign_doc)?;

        // Call eth_signTypedData_v4
        let params = js_sys::Array::of2(&JsValue::from_str(&self.address_hex), &payload);
        let request_obj = Object::new();
        Reflect::set(
            &request_obj,
            &JsValue::from_str("method"),
            &JsValue::from_str("eth_signTypedData_v4"),
        )
        .map_err(|_| SigningError::wallet_adapter("failed to set method"))?;
        Reflect::set(&request_obj, &JsValue::from_str("params"), &params)
            .map_err(|_| SigningError::wallet_adapter("failed to set params"))?;

        let promise = ethereum.request(&request_obj.into());
        let result = JsFuture::from(promise).await.map_err(|e| {
            SigningError::wallet_adapter(format!("eth_signTypedData_v4 failed: {e:?}"))
        })?;

        let sig_hex: String = result.as_string().ok_or_else(|| {
            SigningError::wallet_adapter("MetaMask returned non-string signature")
        })?;

        // Parse the 65-byte recoverable signature (r || s || v) → take r || s (64 bytes)
        let sig_hex = sig_hex.strip_prefix("0x").unwrap_or(&sig_hex);
        let sig_bytes = hex::decode(sig_hex)
            .map_err(|e| SigningError::wallet_adapter(format!("invalid signature hex: {e}")))?;

        if sig_bytes.len() < 64 {
            return Err(SigningError::wallet_adapter(format!(
                "MetaMask signature too short: {} bytes (expected ≥64)",
                sig_bytes.len()
            )));
        }

        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes[..64]);
        Ok(Signature::Secp256k1(arr))
    }

    /// Returns the cached EVM public key placeholder.
    ///
    /// MetaMask does not expose the raw secp256k1 public key directly —
    /// only the 20-byte address. The chain recovers the full public key
    /// from the ECDSA signature (ecrecover) during verification.
    pub(crate) fn public_key(&self) -> PublicKey {
        let addr = *self.cached_address.borrow();
        // Encode as compressed secp256k1 placeholder: [0x02 | address | zeros]
        // The chain resolves the real key via signature recovery.
        let mut key = [0u8; 33];
        key[0] = 0x02; // compressed even-y prefix
        key[1..21].copy_from_slice(&addr);
        PublicKey::Secp256k1(key)
    }

    /// Returns the protobuf-encoded public key for `SignerInfo`.
    pub(crate) fn public_key_proto(&self) -> morpheum_signing_core::Any {
        morpheum_signing_core::Any {
            type_url: "/morpheum.crypto.secp256k1.PubKey".to_string(),
            value: self.cached_address.borrow().to_vec(),
        }
    }

    /// Derives the `AccountId` from the cached EVM address.
    pub(crate) fn account_id(&self) -> AccountId {
        let addr = *self.cached_address.borrow();
        Address::Evm(addr).to_account_id()
    }

    // ==================== PRIVATE HELPERS ====================

    /// Builds a canonical EIP-712 typed data payload from the `SignDoc`.
    fn build_eip712_payload(&self, sign_doc: &SignDoc) -> Result<JsValue, SigningError> {
        let hash = Sha256::digest(sign_doc.encode_to_vec());
        let hash_hex = format!("0x{}", hex::encode(hash));

        // Domain
        let domain = Object::new();
        set_prop(&domain, "name", &JsValue::from_str("Morpheum"))?;
        set_prop(&domain, "version", &JsValue::from_str("1"))?;
        set_prop(&domain, "chainId", &JsValue::from(1))?;

        // Message
        let message = Object::new();
        set_prop(&message, "signDocHash", &JsValue::from_str(&hash_hex))?;

        // Types
        let eip712_domain_type = js_sys::Array::new();
        let name_field = Object::new();
        set_prop(&name_field, "name", &JsValue::from_str("name"))?;
        set_prop(&name_field, "type", &JsValue::from_str("string"))?;
        eip712_domain_type.push(&name_field);
        let version_field = Object::new();
        set_prop(&version_field, "name", &JsValue::from_str("version"))?;
        set_prop(&version_field, "type", &JsValue::from_str("string"))?;
        eip712_domain_type.push(&version_field);
        let chain_id_field = Object::new();
        set_prop(&chain_id_field, "name", &JsValue::from_str("chainId"))?;
        set_prop(&chain_id_field, "type", &JsValue::from_str("uint256"))?;
        eip712_domain_type.push(&chain_id_field);

        let sign_doc_type = js_sys::Array::new();
        let hash_field = Object::new();
        set_prop(&hash_field, "name", &JsValue::from_str("signDocHash"))?;
        set_prop(&hash_field, "type", &JsValue::from_str("string"))?;
        sign_doc_type.push(&hash_field);

        let types = Object::new();
        set_prop(&types, "EIP712Domain", &eip712_domain_type)?;
        set_prop(&types, "MorpheumSignDoc", &sign_doc_type)?;

        // Top-level payload
        let payload = Object::new();
        set_prop(&payload, "types", &types)?;
        set_prop(&payload, "domain", &domain)?;
        set_prop(
            &payload,
            "primaryType",
            &JsValue::from_str("MorpheumSignDoc"),
        )?;
        set_prop(&payload, "message", &message)?;

        js_sys::JSON::stringify(&payload)
            .map(Into::into)
            .map_err(|_| SigningError::wallet_adapter("failed to stringify EIP-712 payload"))
    }
}

/// Tiny helper to reduce `Reflect::set` boilerplate.
fn set_prop(obj: &Object, key: &str, val: &JsValue) -> Result<(), SigningError> {
    Reflect::set(obj, &JsValue::from_str(key), val)
        .map_err(|_| SigningError::wallet_adapter(format!("failed to set property '{key}'")))?;
    Ok(())
}
