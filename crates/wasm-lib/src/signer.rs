//! WasmSigner dispatch enum — static dispatch across all supported browser wallets.

use async_trait::async_trait;

use morpheum_signing_core::{
    error::SigningError,
    proto::tx::v1::SignDoc,
    signer::Signer,
    types::{AccountId, PublicKey, Signature, WalletType},
};

use crate::{MetaMaskAdapterWasm, PhantomAdapterWasm, TaprootAdapterWasm};

/// Static-dispatch signer enum for all supported WASM wallet adapters.
///
/// Used by [`TxBuilderWasm`] and [`MorpheumSdkWasm`] instead of `Box<dyn Signer>`
/// to avoid dynamic dispatch overhead within the WASM boundary.
pub enum WasmSigner {
    MetaMask(MetaMaskAdapterWasm),
    Phantom(PhantomAdapterWasm),
    Taproot(TaprootAdapterWasm),
}

// SAFETY: WASM (wasm32-unknown-unknown) is single-threaded by specification.
unsafe impl Send for WasmSigner {}
unsafe impl Sync for WasmSigner {}

#[async_trait(?Send)]
impl Signer for WasmSigner {
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        match self {
            Self::MetaMask(a) => a.sign_impl(sign_doc).await,
            Self::Phantom(a) => a.sign_impl(sign_doc).await,
            Self::Taproot(a) => a.sign_impl(sign_doc).await,
        }
    }

    fn public_key(&self) -> PublicKey {
        match self {
            Self::MetaMask(a) => a.public_key(),
            Self::Phantom(a) => a.public_key(),
            Self::Taproot(a) => a.public_key(),
        }
    }

    fn wallet_type(&self) -> WalletType {
        match self {
            Self::MetaMask(_) => WalletType::Evm,
            Self::Phantom(_) => WalletType::Solana,
            Self::Taproot(_) => WalletType::Bitcoin,
        }
    }

    fn account_id(&self) -> AccountId {
        match self {
            Self::MetaMask(a) => a.account_id(),
            Self::Phantom(a) => a.account_id(),
            Self::Taproot(a) => a.account_id(),
        }
    }

    fn public_key_proto(&self) -> morpheum_signing_core::Any {
        match self {
            Self::MetaMask(a) => a.public_key_proto(),
            Self::Phantom(a) => a.public_key_proto(),
            Self::Taproot(a) => a.public_key_proto(),
        }
    }
}
