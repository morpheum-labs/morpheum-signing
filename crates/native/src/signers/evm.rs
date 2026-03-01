//! EvmSigner — Local secp256k1 signer for EVM (MetaMask-style) compatibility.
//!
//! This signer is used when a user wants to sign locally with an EVM private key
//! (e.g. for testing, CLI tools, or headless bots). For injected wallets like MetaMask,
//! use the `MetaMaskAdapter` instead (delegates signing to the browser wallet).

use async_trait::async_trait;
use k256::ecdsa::{
    signature::hazmat::PrehashSigner,
    SigningKey as SecpSigningKey, VerifyingKey as SecpVerifyingKey,
    Signature as SecpSignature,
};
use prost::Message;
use zeroize::ZeroizeOnDrop;

use morpheum_signing_core::{
    error::{CryptoError, SigningError},
    proto::tx::v1::SignDoc,
    signer::Signer,
    types::{PublicKey, Signature, WalletType},
};

/// Local secp256k1 signer for EVM compatibility.
///
/// Uses a 32-byte seed (recommended: derive from secure mnemonic or RNG).
#[derive(Debug, Clone)]
pub struct EvmSigner {
    signing_key: SecpSigningKey,
    verifying_key: SecpVerifyingKey,
}

impl EvmSigner {
    /// Creates a new `EvmSigner` from a 32-byte seed.
    ///
    /// In production, use a secure source (mnemonic, hardware, etc.).
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SecpSigningKey::from_slice(seed).expect("Invalid secp256k1 seed");
        let verifying_key = *signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }
}

#[async_trait]
impl Signer for EvmSigner {
    /// Signs the canonical `SignDoc` using secp256k1 and returns `Signature::Secp256k1`.
    ///
    /// # Constant-Time Guarantees
    ///
    /// The `k256` crate performs ECDSA signing with constant-time scalar arithmetic
    /// (via the `crypto-bigint` crate). Secret key material is never compared or
    /// branched on in variable time.
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let signature: SecpSignature = self.signing_key.sign_prehash(&bytes)
            .map_err(|e: k256::ecdsa::Error| {
                SigningError::Crypto(CryptoError::Secp256k1(e.to_string()))
            })?;

        // Return 64-byte (r, s) signature (standard compact form).
        // Recovery ID can be added later if needed for full EVM recoverable signatures.
        Ok(Signature::Secp256k1(signature.to_bytes().into()))
    }

    /// Returns the secp256k1 compressed public key (33 bytes).
    fn public_key(&self) -> PublicKey {
        let compressed = self.verifying_key.to_encoded_point(true);
        let bytes: [u8; 33] = compressed.as_bytes().try_into()
            .expect("secp256k1 compressed public key must be 33 bytes");
        PublicKey::Secp256k1(bytes)
    }

    /// Returns the wallet type for this signer.
    fn wallet_type(&self) -> WalletType {
        WalletType::Evm
    }
}

// k256 `SigningKey` handles its own zeroization on `Drop`.
impl ZeroizeOnDrop for EvmSigner {}