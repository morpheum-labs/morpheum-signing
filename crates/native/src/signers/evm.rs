//! EvmSigner — Local secp256k1 signer for EVM (MetaMask-style) compatibility.
//!
//! This signer is used when a user wants to sign locally with an EVM private key
//! (e.g. for testing, CLI tools, or headless bots). For injected wallets like MetaMask,
//! use the `MetaMaskAdapter` instead (delegates signing to the browser wallet).

use async_trait::async_trait;
use k256::ecdsa::{
    signature::hazmat::PrehashSigner, Signature as SecpSignature, SigningKey as SecpSigningKey,
    VerifyingKey as SecpVerifyingKey,
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

/// Standard Ethereum BIP-44 derivation path: `m/44'/60'/0'/0/0`.
pub const EVM_DEFAULT_PATH: [u32; 5] = [44 | 0x8000_0000, 60 | 0x8000_0000, 0x8000_0000, 0, 0];

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

    /// Creates a new `EvmSigner` from a BIP-39 mnemonic phrase.
    ///
    /// Derives the secp256k1 key at the standard Ethereum path `m/44'/60'/0'/0/0`
    /// via BIP-32 HD derivation (HMAC-SHA512 based).
    ///
    /// # Parameters
    ///
    /// - `mnemonic`: A valid BIP-39 mnemonic (12, 15, 18, 21, or 24 words).
    /// - `passphrase`: BIP-39 passphrase. Use `""` for the default.
    #[cfg(feature = "bip39")]
    pub fn from_mnemonic(mnemonic: &str, passphrase: &str) -> Result<Self, SigningError> {
        Self::from_mnemonic_with_index(mnemonic, passphrase, 0)
    }

    /// Like [`from_mnemonic`](Self::from_mnemonic) but derives at
    /// `m/44'/60'/0'/0/{index}`, allowing multiple accounts from one mnemonic.
    #[cfg(feature = "bip39")]
    pub fn from_mnemonic_with_index(
        mnemonic: &str,
        passphrase: &str,
        index: u32,
    ) -> Result<Self, SigningError> {
        use zeroize::Zeroize;

        let parsed = bip39::Mnemonic::parse(mnemonic)
            .map_err(|e| SigningError::invalid_key(format!("invalid BIP-39 mnemonic: {e}")))?;

        let bip39_seed = parsed.to_seed(passphrase);
        let path = [44 | 0x8000_0000, 60 | 0x8000_0000, 0x8000_0000, 0, index];

        let mut key_bytes = bip32_derive(&bip39_seed, &path)
            .map_err(|e| SigningError::invalid_key(format!("BIP-32 derivation failed: {e}")))?;

        let signer = Self::from_seed(&key_bytes);
        key_bytes.zeroize();

        Ok(signer)
    }

    /// Returns the raw 32-byte private key.
    ///
    /// The caller is responsible for zeroizing this material after use.
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes().into()
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
        let signature: SecpSignature =
            self.signing_key
                .sign_prehash(&bytes)
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
        let bytes: [u8; 33] = compressed
            .as_bytes()
            .try_into()
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

// ── BIP-32 HD key derivation (HMAC-SHA512) ──────────────────────────

/// Derives a 32-byte child private key from a BIP-39 seed using BIP-32 HD
/// derivation. Supports both hardened (index >= 0x8000_0000) and normal child
/// key derivation.
fn bip32_derive(seed: &[u8], path: &[u32]) -> Result<[u8; 32], &'static str> {
    use hmac::{Hmac, Mac};
    use k256::elliptic_curve::PrimeField;
    use sha2::Sha512;

    type HmacSha512 = Hmac<Sha512>;

    let mut mac =
        HmacSha512::new_from_slice(b"Bitcoin seed").map_err(|_| "HMAC key creation failed")?;
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 32];
    let mut chain_code = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain_code.copy_from_slice(&result[32..]);

    for &index in path {
        let mut mac =
            HmacSha512::new_from_slice(&chain_code).map_err(|_| "HMAC key creation failed")?;

        if index & 0x8000_0000 != 0 {
            // Hardened child: 0x00 || key || index
            mac.update(&[0x00]);
            mac.update(&key);
        } else {
            // Normal child: compressed_pubkey || index
            let sk = SecpSigningKey::from_slice(&key).map_err(|_| "invalid derived key")?;
            let pk = sk.verifying_key().to_encoded_point(true);
            mac.update(pk.as_bytes());
        }
        mac.update(&index.to_be_bytes());

        let result = mac.finalize().into_bytes();
        let il = &result[..32];

        // child_key = parse256(IL) + parent_key (mod n)
        let parent: k256::Scalar =
            Option::from(k256::Scalar::from_repr(key.into())).ok_or("invalid parent key scalar")?;

        let mut il_arr = [0u8; 32];
        il_arr.copy_from_slice(il);
        let tweak: k256::Scalar =
            Option::from(k256::Scalar::from_repr(il_arr.into())).ok_or("invalid tweak scalar")?;

        let child = parent + tweak;
        if bool::from(child.is_zero()) {
            return Err("derived key is zero");
        }
        key = child.to_repr().into();
        chain_code.copy_from_slice(&result[32..]);
    }

    Ok(key)
}
