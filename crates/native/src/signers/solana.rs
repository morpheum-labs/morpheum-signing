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