//! SolanaSigner — Local ed25519 keypair signer for Solana compatibility.
//!
//! This signer provides **local signing** using the exact ed25519 curve and key format
//! used by Solana (32-byte private key / seed). It is intended for:
//! - CLI tools, bots, tests, or headless server environments
//! - Cases where you directly control the Solana private key
//!
//! For **browser-injected** Phantom (or Solflare) wallet signing, use
//! `PhantomAdapter` (in `adapters/phantom.rs`) instead. The adapter pattern
//! handles the injected `window.phantom` API while this signer is purely local.
//!
//! Design invariants (aligned with the rest of the native signers):
//! - Uses `ed25519-dalek` (battle-tested, Solana's official library).
//! - Returns `Signature::Ed25519` and `PublicKey::Ed25519` (matches Solana pubkey/sig format).
//! - Full security: `Zeroize` + `ZeroizeOnDrop` on all secret material.
//! - Implements the core `Signer` trait with zero-cost abstraction.
//! - `WalletType::Solana` for correct nonce strategy and address mapping in `TxBuilder`.

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

/// Standard Solana BIP-44 derivation path: `m/44'/501'/0'/0'`.
///
/// All segments are hardened, per the SLIP-0010 Ed25519 requirement.
pub const SOLANA_DEFAULT_PATH: [u32; 4] = [
    44 | 0x8000_0000,
    501 | 0x8000_0000,
    0x8000_0000,
    0x8000_0000,
];

/// Local ed25519 signer optimized for Solana.
///
/// Holds a standard 32-byte ed25519 private key (seed). This is the canonical
/// format used by Solana wallets and tools (e.g. `solana-keygen`).
#[derive(Debug, Clone)]
pub struct SolanaSigner {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl SolanaSigner {
    /// Creates a new `SolanaSigner` from a 32-byte seed (standard Solana private key format).
    ///
    /// **Security recommendation**: In production, the seed should come from a secure source:
    /// - BIP-39 mnemonic + derivation path (via `bip39` + `ed25519-dalek`)
    /// - Hardware wallet export
    /// - Secure key derivation (PBKDF2 / Argon2)
    ///
    /// Never hardcode or commit seeds to version control.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Creates a new `SolanaSigner` from a BIP-39 mnemonic phrase.
    ///
    /// Derives the Ed25519 key at the standard Solana path `m/44'/501'/0'/0'`
    /// via SLIP-0010 HD derivation (HMAC-SHA512 based, hardened-only).
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
    /// `m/44'/501'/{index}'/0'`, allowing multiple accounts from one mnemonic.
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
        let path = [
            44 | 0x8000_0000,
            501 | 0x8000_0000,
            index | 0x8000_0000,
            0x8000_0000,
        ];

        let mut key_bytes = slip0010_derive(&bip39_seed, &path)
            .map_err(|e| SigningError::invalid_key(format!("SLIP-0010 derivation failed: {e}")))?;

        let signer = Self::from_seed(&key_bytes);
        key_bytes.zeroize();

        Ok(signer)
    }

    /// Returns the raw 32-byte ed25519 public key (Solana address bytes).
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Returns the raw 32-byte private key (seed).
    ///
    /// The caller is responsible for zeroizing this material after use.
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
}

#[async_trait]
impl Signer for SolanaSigner {
    /// Signs the canonical `SignDoc` using ed25519 (Solana-compatible signature).
    ///
    /// The resulting 64-byte signature is exactly what Solana nodes and Morpheum's
    /// Solana compatibility layer expect.
    ///
    /// # Constant-Time Guarantees
    ///
    /// Uses `ed25519-dalek` which performs constant-time signing with respect to
    /// the secret key material. See [`NativeSigner::sign`] for details.
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let signature = self.signing_key.sign(&bytes);
        Ok(Signature::Ed25519(signature.to_bytes()))
    }

    /// Returns the ed25519 public key (32 bytes) in the exact format used by Solana.
    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.verifying_key.to_bytes())
    }

    /// Returns `WalletType::Solana` — used by `TxBuilder` / `AddressMapper` for:
    /// - Correct nonce strategy (usually sequential for human-like flows)
    /// - Address mapping (`sol...` base58 → canonical `AccountId`)
    fn wallet_type(&self) -> WalletType {
        WalletType::Solana
    }
}

// `ed25519_dalek::SigningKey` handles its own zeroization on `Drop`.
impl ZeroizeOnDrop for SolanaSigner {}

// ── SLIP-0010 Ed25519 HD key derivation (hardened-only) ─────────────

/// Derives a 32-byte child private key from a BIP-39 seed using SLIP-0010
/// for Ed25519. Only hardened derivation is supported (all path indices
/// must have the 0x8000_0000 bit set).
fn slip0010_derive(seed: &[u8], path: &[u32]) -> Result<[u8; 32], &'static str> {
    use hmac::{Hmac, Mac};
    use sha2::Sha512;

    type HmacSha512 = Hmac<Sha512>;

    let mut mac =
        HmacSha512::new_from_slice(b"ed25519 seed").map_err(|_| "HMAC key creation failed")?;
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 32];
    let mut chain_code = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain_code.copy_from_slice(&result[32..]);

    for &index in path {
        if index & 0x8000_0000 == 0 {
            return Err("SLIP-0010 Ed25519 only supports hardened derivation");
        }

        let mut mac =
            HmacSha512::new_from_slice(&chain_code).map_err(|_| "HMAC key creation failed")?;
        mac.update(&[0x00]);
        mac.update(&key);
        mac.update(&index.to_be_bytes());

        let result = mac.finalize().into_bytes();
        key.copy_from_slice(&result[..32]);
        chain_code.copy_from_slice(&result[32..]);
    }

    Ok(key)
}
