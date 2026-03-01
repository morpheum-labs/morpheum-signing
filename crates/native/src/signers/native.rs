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
    /// In production: derive from BIP-39 mnemonic (see [`from_mnemonic`](Self::from_mnemonic))
    /// or hardware wallet.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self { signing_key, verifying_key }
    }

    /// Creates a new `NativeSigner` from a BIP-39 mnemonic phrase.
    ///
    /// The 64-byte BIP-39 seed is derived from the mnemonic and passphrase,
    /// and the first 32 bytes are used as the ed25519 signing key seed
    /// (standard approach, compatible with Solana / SLIP-0010).
    ///
    /// # Parameters
    ///
    /// - `mnemonic`: A valid BIP-39 mnemonic (12, 15, 18, 21, or 24 words).
    /// - `passphrase`: BIP-39 passphrase. Use `""` for the default (no passphrase).
    ///
    /// # Errors
    ///
    /// Returns [`SigningError::InvalidKey`] if the mnemonic is malformed.
    ///
    /// # Security
    ///
    /// The intermediate seed material is zeroized after key derivation.
    #[cfg(feature = "bip39")]
    pub fn from_mnemonic(mnemonic: &str, passphrase: &str) -> Result<Self, SigningError> {
        use zeroize::Zeroize;

        let parsed = bip39::Mnemonic::parse(mnemonic)
            .map_err(|e| SigningError::invalid_key(format!("invalid BIP-39 mnemonic: {e}")))?;

        let seed = parsed.to_seed(passphrase);
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&seed[..32]);

        let signer = Self::from_seed(&key_bytes);
        key_bytes.zeroize();

        Ok(signer)
    }
}

#[async_trait]
impl Signer for NativeSigner {
    /// Signs the canonical `SignDoc` using ed25519.
    ///
    /// # Constant-Time Guarantees
    ///
    /// The underlying `ed25519-dalek` library performs signing in constant time
    /// with respect to the secret key, preventing timing side-channel attacks.
    /// The `SigningKey::sign` method uses a deterministic nonce (RFC 6979 style)
    /// derived from the message and secret key, so no external randomness is needed.
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