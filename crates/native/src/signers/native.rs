//! NativeSigner — Local ed25519 keypair signer for Morpheum's native curve.
//!
//! This is the standard signer for Morpheum native accounts (ed25519).
//! It is the recommended signer for most human users who control their own
//! native private key (CLI, bots, headless environments, etc.).
//!
//! For chain-specific local signing, use EvmSigner, SolanaSigner, or BitcoinSigner.
//! For injected wallets, use the adapters in `adapters/`.

use async_trait::async_trait;
use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use prost::Message;
use zeroize::ZeroizeOnDrop;

use morpheum_signing_core::{
    error::SigningError,
    proto::tx::v1::SignDoc,
    signer::Signer,
    types::{PublicKey, Signature, WalletType},
};

/// Local ed25519 signer for Morpheum native accounts.
#[derive(Debug, Clone)]
pub struct NativeSigner {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl NativeSigner {
    /// Creates a new `NativeSigner` from a 32-byte seed.
    ///
    /// In production: derive from BIP-39 mnemonic or hardware wallet.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self { signing_key, verifying_key }
    }
}

#[async_trait]
impl Signer for NativeSigner {
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let signature = self.signing_key.sign(&bytes);
        Ok(Signature::Ed25519(signature.to_bytes()))
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.verifying_key.to_bytes())
    }

    fn wallet_type(&self) -> WalletType {
        WalletType::Native
    }
}

// `ed25519_dalek::SigningKey` implements `Drop` which zeroizes secret material,
// so no manual `Zeroize` call is needed — just propagate the trait marker.
impl ZeroizeOnDrop for NativeSigner {}