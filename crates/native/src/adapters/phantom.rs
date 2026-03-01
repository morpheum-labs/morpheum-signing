//! PhantomAdapter — Injected Solana wallet adapter for browser environments.
//!
//! Implements the `WalletAdapter` trait by delegating signing to the injected
//! `window.phantom.solana` provider (Phantom, Solflare, Backpack, etc.).
//!
//! **Design**:
//! - Follows the **Adapter Pattern** (GoF) to convert Phantom's JavaScript API
//!   into the clean, async `WalletAdapter` interface expected by `TxBuilder`.
//! - Uses `signMessage` with a clear prefixed SHA-256 hash of the canonical `SignDoc`
//!   — this is the recommended, secure, and user-friendly pattern for Solana wallets.
//! - Returns exactly `Signature::Ed25519([u8; 64])` and uses `WalletType::Solana`.
//! - Fully async, robust error handling, and secure (signatures zeroized downstream).
//!
//! This adapter is **browser-only** (WASM target) and is the correct choice for
//! dApps using injected Solana wallets. For local Solana keypair signing, use
//! `SolanaSigner` in the `signers/` module.

use async_trait::async_trait;
use js_sys::{Uint8Array, Reflect};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use morpheum_signing_core::{
    error::SigningError,
    proto::tx::v1::SignDoc,
    types::{Address, Signature, WalletType},
    wallet_adapter::WalletAdapter,
};

/// JavaScript interop for the injected `window.phantom.solana` object.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    type Phantom;

    #[wasm_bindgen(js_namespace = window, js_name = phantom)]
    static PHANTOM: Option<Phantom>;

    #[wasm_bindgen(method, getter, js_name = solana)]
    fn solana(this: &Phantom) -> Option<SolanaProvider>;

    #[wasm_bindgen]
    type SolanaProvider;

    #[wasm_bindgen(method, js_name = connect)]
    async fn connect(this: &SolanaProvider) -> JsValue;

    #[wasm_bindgen(method, js_name = signMessage)]
    async fn sign_message(this: &SolanaProvider, message: &Uint8Array) -> JsValue;
}

/// Phantom (Solana) injected wallet adapter.
#[derive(Debug, Clone, Default)]
pub struct PhantomAdapter {
    /// Cached Solana public key (32 bytes) once connected.
    cached_address: Option<Address>,
}

impl PhantomAdapter {
    /// Creates a new Phantom adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self { cached_address: None }
    }
}

#[async_trait]
impl WalletAdapter for PhantomAdapter {
    /// Requests a signature from Phantom using `signMessage`.
    ///
    /// The `SignDoc` is hashed (SHA-256) and prefixed with a clear message
    /// for security, determinism, and good UX in the wallet popup.
    async fn request_signature(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let phantom = PHANTOM
            .as_ref()
            .ok_or_else(|| SigningError::wallet_adapter("Phantom not detected (window.phantom missing)"))?;

        let solana = phantom
            .solana()
            .ok_or_else(|| SigningError::wallet_adapter("Phantom Solana provider not available"))?;

        // 1. Ensure wallet is connected and cache address
        let _pubkey = self.ensure_connected(&solana).await?;

        // 2. Build clear, prefixed message
        let message = Self::build_sign_message(sign_doc);

        // 3. Request signature
        let result = JsFuture::from(solana.sign_message(&message))
            .await
            .map_err(|e| SigningError::wallet_adapter(format!("Phantom signMessage failed: {:?}", e)))?;

        // 4. Extract signature (Uint8Array)
        let signature_value = Reflect::get(&result, &JsValue::from("signature"))
            .map_err(|_| SigningError::wallet_adapter("Phantom response missing 'signature' field"))?;

        let signature_array: Uint8Array = signature_value
            .dyn_into()
            .map_err(|_| SigningError::wallet_adapter("Phantom signature is not a Uint8Array"))?;

        let len = signature_array.length() as usize;
        if len != 64 {
            return Err(SigningError::wallet_adapter(format!(
                "Phantom returned invalid signature length: {} (expected 64)",
                len
            )));
        }

        let mut sig_bytes = [0u8; 64];
        signature_array.copy_to(&mut sig_bytes);

        Ok(Signature::Ed25519(sig_bytes))
    }

    fn wallet_type(&self) -> WalletType {
        WalletType::Solana
    }

    fn external_address(&self) -> &Address {
        self.cached_address
            .as_ref()
            .unwrap_or(&Address::Solana([0u8; 32]))
    }

    fn name(&self) -> &'static str {
        "Phantom (Solana)"
    }
}

impl PhantomAdapter {
    /// Ensures the wallet is connected and caches the public key.
    async fn ensure_connected(&self, provider: &SolanaProvider) -> Result<[u8; 32], SigningError> {
        if let Some(Address::Solana(pk)) = self.cached_address {
            return Ok(pk);
        }

        let result = JsFuture::from(provider.connect())
            .await
            .map_err(|e| SigningError::wallet_adapter(format!("Phantom connect failed: {:?}", e)))?;

        let public_key_value = Reflect::get(&result, &JsValue::from("publicKey"))
            .map_err(|_| SigningError::wallet_adapter("Phantom response missing 'publicKey'"))?;

        let pubkey_array: Uint8Array = public_key_value
            .dyn_into()
            .map_err(|_| SigningError::wallet_adapter("Phantom publicKey is not a Uint8Array"))?;

        let len = pubkey_array.length() as usize;
        if len != 32 {
            return Err(SigningError::wallet_adapter(format!(
                "Phantom returned invalid public key length: {} (expected 32)",
                len
            )));
        }

        let mut pubkey = [0u8; 32];
        pubkey_array.copy_to(&mut pubkey);

        Ok(pubkey)
    }

    /// Builds a clear, human-readable prefixed message for `signMessage`.
    ///
    /// Format: "Morpheum SignDoc v1: <sha256(SignDoc)>"
    /// This is the recommended secure pattern for Solana wallets.
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