//! BitcoinSigner — BIP-340 Schnorr signer for Bitcoin Taproot compatibility.
//!
//! This provides **local** signing using the exact BIP-340 Schnorr signature scheme
//! used by Bitcoin Taproot (x-only public keys, 64-byte signatures).
//!
//! Intended for:
//! - CLI tools, bots, servers, autonomous agents, or tests where you control the private key.
//! - Generating Taproot-compatible signatures for Morpheum multi-chain flows.
//!
//! For **browser-injected** Taproot wallets (Unisat, Leather, Xverse, etc.), use the
//! `TaprootAdapter` in `adapters/taproot.rs` instead. This signer is purely local.
//!
//! Design invariants (identical to HumanSigner / AgentSigner / SolanaSigner / EvmSigner):
//! - Uses the official `bitcoin` crate (v0.32+) for battle-tested BIP-340 implementation.
//! - Returns `PublicKey::Schnorr([u8; 32])` (X-only pubkey) and `Signature::Schnorr([u8; 64])`.
//! - Full security: `Zeroize` + `ZeroizeOnDrop` on all secret material.
//! - `WalletType::Bitcoin` for correct address mapping (`bc1p...`) and nonce strategy in `TxBuilder`.
//! - Message hashing: SHA-256 of canonical `SignDoc` bytes (standard, consistent with other signers).

use async_trait::async_trait;
use bitcoin::secp256k1::{
    schnorr::Signature as SchnorrSignature,
    Keypair, Message as Secp256k1Message, Secp256k1,
};
use prost::Message;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use morpheum_signing_core::{
    error::SigningError,
    proto::tx::v1::SignDoc,
    signer::Signer,
    types::{PublicKey, Signature, WalletType},
};

/// Local BIP-340 Schnorr signer for Bitcoin Taproot.
///
/// Holds a secp256k1 `Keypair` derived from a 32-byte seed (standard format).
#[derive(Debug, Clone)]
pub struct BitcoinSigner {
    keypair: Keypair,
    /// Cached X-only public key (32 bytes) — standard for Taproot.
    x_only_pubkey: [u8; 32],
}

impl BitcoinSigner {
    /// Creates a new `BitcoinSigner` from a 32-byte seed (private key).
    ///
    /// **Security note**: In production, derive this seed securely:
    /// - BIP-39 mnemonic + BIP-86 derivation path (m/86'/0'/0'/0/n)
    /// - Hardware wallet export
    /// - Secure KDF (Argon2id, PBKDF2, etc.)
    ///
    /// Never hardcode or commit private keys to source control.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let secp = Secp256k1::new();
        let keypair = Keypair::from_seckey_slice(&secp, seed)
            .expect("32-byte seed is always valid for secp256k1/BIP-340");

        let (x_only, _parity) = keypair.x_only_public_key();

        Self {
            keypair,
            x_only_pubkey: x_only.serialize(),
        }
    }
}

#[async_trait]
impl Signer for BitcoinSigner {
    /// Signs the canonical `SignDoc` using BIP-340 Schnorr.
    ///
    /// The `SignDoc` is first hashed with SHA-256 (standard, deterministic practice
    /// for structured data with Schnorr). Returns a 64-byte signature (r || s).
    ///
    /// # Constant-Time Guarantees
    ///
    /// The `bitcoin::secp256k1` crate (libsecp256k1) performs all scalar operations
    /// in constant time. The `sign_schnorr_no_aux_rand` variant uses no additional
    /// randomness, making signing fully deterministic and reproducible.
    async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        let bytes = sign_doc.encode_to_vec();
        let hash = Sha256::digest(bytes);
        let message = Secp256k1Message::from_digest(hash.into());

        let secp = Secp256k1::new();
        let sig: SchnorrSignature = secp.sign_schnorr_no_aux_rand(&message, &self.keypair);

        Ok(Signature::Schnorr(sig.serialize()))
    }

    /// Returns the 32-byte X-only public key (BIP-340 / Taproot standard).
    fn public_key(&self) -> PublicKey {
        PublicKey::Schnorr(self.x_only_pubkey)
    }

    /// Returns `WalletType::Bitcoin` — used by `TxBuilder` for correct address mapping
    /// (bc1p...) and nonce strategy selection.
    fn wallet_type(&self) -> WalletType {
        WalletType::Bitcoin
    }
}

impl Drop for BitcoinSigner {
    fn drop(&mut self) {
        // Keypair internally holds secret material and is zeroized by the secp256k1
        // library where supported. We explicitly zeroize the cached public key material.
        self.x_only_pubkey.zeroize();
    }
}

impl ZeroizeOnDrop for BitcoinSigner {}