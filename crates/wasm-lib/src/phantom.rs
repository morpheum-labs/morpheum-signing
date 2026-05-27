//! Phantom (Solana) injected wallet adapter for WASM/browser environments.
//!
//! Delegates signing to `window.phantom.solana` via `signMessage`, the recommended
//! and user-friendly approach for Solana wallets. Returns `Signature::Ed25519`.
//!
//! Uses [`RefCell`] for interior mutability of the cached Solana public key,
//! enabling future account-change handling without `&mut self`.

use std::cell::RefCell;

use js_sys::{Reflect, Uint8Array};
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
    /// Opaque handle to `window.phantom.solana`.
    type SolanaProvider;

    #[wasm_bindgen(method, js_name = "connect")]
    fn connect(this: &SolanaProvider) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = "signMessage")]
    fn sign_message(this: &SolanaProvider, message: &Uint8Array) -> js_sys::Promise;
}

/// Retrieves the injected `window.phantom.solana` provider or returns a clear error.
fn get_phantom_solana() -> Result<SolanaProvider, SigningError> {
    let window = web_sys::window()
        .ok_or_else(|| SigningError::wallet_adapter("no window object (not in a browser?)"))?;

    let phantom = Reflect::get(&window, &JsValue::from_str("phantom"))
        .map_err(|_| SigningError::wallet_adapter("failed to access window.phantom"))?;
    if phantom.is_undefined() || phantom.is_null() {
        return Err(SigningError::wallet_adapter(
            "Phantom not detected (window.phantom is missing)",
        ));
    }

    let solana = Reflect::get(&phantom, &JsValue::from_str("solana"))
        .map_err(|_| SigningError::wallet_adapter("failed to access window.phantom.solana"))?;
    if solana.is_undefined() || solana.is_null() {
        return Err(SigningError::wallet_adapter(
            "Phantom Solana provider not available",
        ));
    }

    Ok(solana.unchecked_into::<SolanaProvider>())
}

// ==================== ADAPTER ====================

/// Phantom (Solana) injected wallet adapter.
///
/// Created via the async [`connect()`](Self::connect) method, which requests
/// wallet connection and caches the ed25519 public key (32 bytes).
pub struct PhantomAdapterWasm {
    /// Mutable cached Solana public key (32 bytes).
    ///
    /// SAFETY: `RefCell` is `!Sync`, but WASM is single-threaded —
    /// no concurrent access is possible.
    cached_pubkey: RefCell<[u8; 32]>,
}

// SAFETY: WASM (wasm32-unknown-unknown) is single-threaded by specification.
unsafe impl Send for PhantomAdapterWasm {}
unsafe impl Sync for PhantomAdapterWasm {}

impl PhantomAdapterWasm {
    /// Connects to Phantom, requests wallet access, and caches the ed25519 public key.
    pub async fn connect() -> Result<Self, SigningError> {
        let provider = get_phantom_solana()?;

        let promise = provider.connect();
        let result = JsFuture::from(promise)
            .await
            .map_err(|e| SigningError::wallet_adapter(format!("Phantom connect failed: {e:?}")))?;

        // Extract publicKey (Uint8Array of 32 bytes)
        let pk_val = Reflect::get(&result, &JsValue::from_str("publicKey"))
            .map_err(|_| SigningError::wallet_adapter("Phantom response missing 'publicKey'"))?;

        // Phantom returns a PublicKey object — call toBytes() to get the raw bytes
        let to_bytes_fn = Reflect::get(&pk_val, &JsValue::from_str("toBytes"))
            .map_err(|_| SigningError::wallet_adapter("publicKey missing toBytes()"))?;
        let pk_bytes_val = if to_bytes_fn.is_function() {
            let func: js_sys::Function = to_bytes_fn.unchecked_into();
            func.call0(&pk_val)
                .map_err(|_| SigningError::wallet_adapter("publicKey.toBytes() failed"))?
        } else {
            pk_val
        };

        let pk_array: Uint8Array = pk_bytes_val
            .dyn_into()
            .map_err(|_| SigningError::wallet_adapter("Phantom publicKey is not a Uint8Array"))?;

        let len = pk_array.length() as usize;
        if len != 32 {
            return Err(SigningError::wallet_adapter(format!(
                "Phantom returned invalid public key length: {len} (expected 32)"
            )));
        }

        let mut pubkey = [0u8; 32];
        pk_array.copy_to(&mut pubkey);

        Ok(Self {
            cached_pubkey: RefCell::new(pubkey),
        })
    }

    /// Signs the canonical `SignDoc` using Phantom's `signMessage`.
    pub(crate) async fn sign_impl(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let provider = get_phantom_solana()?;

        // Build prefixed message: "Morpheum SignDoc v1: <sha256(SignDoc)>"
        let message = Self::build_sign_message(sign_doc);

        let promise = provider.sign_message(&message);
        let result = JsFuture::from(promise).await.map_err(|e| {
            SigningError::wallet_adapter(format!("Phantom signMessage failed: {e:?}"))
        })?;

        // Extract signature Uint8Array
        let sig_val = Reflect::get(&result, &JsValue::from_str("signature"))
            .map_err(|_| SigningError::wallet_adapter("Phantom response missing 'signature'"))?;

        let sig_array: Uint8Array = sig_val
            .dyn_into()
            .map_err(|_| SigningError::wallet_adapter("Phantom signature is not a Uint8Array"))?;

        let len = sig_array.length() as usize;
        if len != 64 {
            return Err(SigningError::wallet_adapter(format!(
                "Phantom returned invalid signature length: {len} (expected 64)"
            )));
        }

        let mut sig_bytes = [0u8; 64];
        sig_array.copy_to(&mut sig_bytes);
        Ok(Signature::Ed25519(sig_bytes))
    }

    /// Returns the cached ed25519 public key.
    pub(crate) fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(*self.cached_pubkey.borrow())
    }

    /// Returns the protobuf-encoded public key for `SignerInfo`.
    pub(crate) fn public_key_proto(&self) -> morpheum_signing_core::Any {
        morpheum_signing_core::Any {
            type_url: "/morpheum.crypto.ed25519.PubKey".to_string(),
            value: self.cached_pubkey.borrow().to_vec(),
        }
    }

    /// Derives the `AccountId` from the cached Solana public key.
    pub(crate) fn account_id(&self) -> AccountId {
        let pk = *self.cached_pubkey.borrow();
        Address::Solana(pk).to_account_id()
    }

    // ==================== PRIVATE HELPERS ====================

    /// Builds a clear, human-readable prefixed message for `signMessage`.
    ///
    /// Format: `"Morpheum SignDoc v1: " || sha256(SignDoc)`
    fn build_sign_message(sign_doc: &SignDoc) -> Uint8Array {
        let bytes = sign_doc.encode_to_vec();
        let hash = Sha256::digest(bytes);
        let prefix = b"Morpheum SignDoc v1: ";

        let mut message = Vec::with_capacity(prefix.len() + hash.len());
        message.extend_from_slice(prefix);
        message.extend_from_slice(&hash);

        Uint8Array::from(message.as_slice())
    }
}
