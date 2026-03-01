//! Full signing flow tests for every signer type (EVM, Solana, Bitcoin).
//!
//! These tests verify that each signer successfully produces a `SignedTx`
//! with correct signatures, signer info, and transaction structure.

use super::common::*;
use morpheum_signing_core::signer::Signer;
use morpheum_signing_native::prelude::*;

fn test_message() -> Any {
    Any {
        type_url: "type.googleapis.com/test.v1.MsgTest".to_string(),
        value: vec![1, 2, 3, 4],
    }
}

// ==================== EVM SIGNING FLOW ====================

#[tokio::test]
async fn test_evm_full_signing_flow() {
    let signer = EvmSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 10 };

    let signed_tx = evm(signer)
        .chain_id("morpheum-mainnet-1")
        .memo("EVM signing test")
        .with_nonce_provider(nonce_provider)
        .add_message(test_message())
        .sign()
        .await
        .expect("EVM signing failed");

    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(signed_tx.tx.signatures.len(), 1);
    assert!(!signed_tx.tx.signatures[0].is_empty());
    assert_eq!(signed_tx.tx.body.as_ref().unwrap().memo, "EVM signing test");
}

#[tokio::test]
async fn test_evm_signature_is_64_bytes() {
    let signer = EvmSigner::from_seed(&TEST_SEED);

    let signed_tx = evm(signer)
        .chain_id("morpheum-test-1")
        .add_message(test_message())
        .sign()
        .await
        .unwrap();

    // secp256k1 compact signature is 64 bytes
    assert_eq!(signed_tx.tx.signatures[0].len(), 64);
}

// ==================== SOLANA SIGNING FLOW ====================

#[tokio::test]
async fn test_solana_full_signing_flow() {
    let signer = SolanaSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 20 };

    let signed_tx = solana(signer)
        .chain_id("morpheum-mainnet-1")
        .memo("Solana signing test")
        .with_nonce_provider(nonce_provider)
        .add_message(test_message())
        .sign()
        .await
        .expect("Solana signing failed");

    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(signed_tx.tx.signatures.len(), 1);
    assert!(!signed_tx.tx.signatures[0].is_empty());
}

#[tokio::test]
async fn test_solana_signature_is_64_bytes() {
    let signer = SolanaSigner::from_seed(&TEST_SEED);

    let signed_tx = solana(signer)
        .chain_id("morpheum-test-1")
        .add_message(test_message())
        .sign()
        .await
        .unwrap();

    // ed25519 signature is 64 bytes
    assert_eq!(signed_tx.tx.signatures[0].len(), 64);
}

// ==================== BITCOIN SIGNING FLOW ====================

#[tokio::test]
async fn test_bitcoin_full_signing_flow() {
    let signer = BitcoinSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 30 };

    let signed_tx = bitcoin(signer)
        .chain_id("morpheum-mainnet-1")
        .memo("Bitcoin Taproot signing test")
        .with_nonce_provider(nonce_provider)
        .add_message(test_message())
        .sign()
        .await
        .expect("Bitcoin signing failed");

    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(signed_tx.tx.signatures.len(), 1);
    assert!(!signed_tx.tx.signatures[0].is_empty());
}

#[tokio::test]
async fn test_bitcoin_signature_is_64_bytes() {
    let signer = BitcoinSigner::from_seed(&TEST_SEED);

    let signed_tx = bitcoin(signer)
        .chain_id("morpheum-test-1")
        .add_message(test_message())
        .sign()
        .await
        .unwrap();

    // BIP-340 Schnorr signature is 64 bytes
    assert_eq!(signed_tx.tx.signatures[0].len(), 64);
}

// ==================== CROSS-SIGNER TESTS ====================

#[tokio::test]
async fn test_different_signers_produce_different_signatures() {
    let msg = test_message();

    let native_tx = native(NativeSigner::from_seed(&TEST_SEED))
        .chain_id("morpheum-test-1")
        .add_message(msg.clone())
        .sign()
        .await
        .unwrap();

    let evm_tx = evm(EvmSigner::from_seed(&TEST_SEED))
        .chain_id("morpheum-test-1")
        .add_message(msg.clone())
        .sign()
        .await
        .unwrap();

    let btc_tx = bitcoin(BitcoinSigner::from_seed(&TEST_SEED))
        .chain_id("morpheum-test-1")
        .add_message(msg)
        .sign()
        .await
        .unwrap();

    // Different curves → different signatures (even if same seed)
    assert_ne!(native_tx.tx.signatures, evm_tx.tx.signatures);
    assert_ne!(native_tx.tx.signatures, btc_tx.tx.signatures);
    assert_ne!(evm_tx.tx.signatures, btc_tx.tx.signatures);
}

#[tokio::test]
async fn test_signed_tx_has_tx_raw() {
    let signer = NativeSigner::from_seed(&TEST_SEED);

    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .add_message(test_message())
        .sign()
        .await
        .unwrap();

    assert!(signed_tx.tx_raw().is_some(), "SignedTx should contain TxRaw");
    let tx_raw = signed_tx.tx_raw().unwrap();
    assert!(!tx_raw.body_bytes.is_empty());
    assert!(!tx_raw.auth_info_bytes.is_empty());
    assert_eq!(tx_raw.signatures.len(), 1);
}

#[tokio::test]
async fn test_txhash_hex_is_valid() {
    let signer = NativeSigner::from_seed(&TEST_SEED);

    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .add_message(test_message())
        .sign()
        .await
        .unwrap();

    let txhash = signed_tx.txhash_hex();
    assert_eq!(txhash.len(), 64, "SHA-256 hex should be 64 chars");
    assert!(txhash.chars().all(|c| c.is_ascii_hexdigit()), "txhash should be valid hex");
}

// ==================== BIP-39 MNEMONIC TESTS ====================

#[cfg(feature = "bip39")]
mod bip39_tests {
    use super::*;

    #[tokio::test]
    async fn test_from_mnemonic_valid() {
        // Standard 12-word BIP-39 test mnemonic
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let signer = NativeSigner::from_mnemonic(mnemonic, "")
            .expect("Valid mnemonic should succeed");

        let signed_tx = native(signer)
            .chain_id("morpheum-test-1")
            .add_message(test_message())
            .sign()
            .await
            .expect("Mnemonic-derived signer should sign successfully");

        assert!(!signed_tx.raw_bytes.is_empty());
    }

    #[test]
    fn test_from_mnemonic_invalid() {
        let result = NativeSigner::from_mnemonic("invalid mnemonic words", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_mnemonic_deterministic() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let s1 = NativeSigner::from_mnemonic(mnemonic, "").unwrap();
        let s2 = NativeSigner::from_mnemonic(mnemonic, "").unwrap();
        assert_eq!(s1.public_key(), s2.public_key(), "Same mnemonic should produce same key");
    }

    #[test]
    fn test_from_mnemonic_passphrase_changes_key() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let s1 = NativeSigner::from_mnemonic(mnemonic, "").unwrap();
        let s2 = NativeSigner::from_mnemonic(mnemonic, "secret").unwrap();
        assert_ne!(
            s1.public_key(),
            s2.public_key(),
            "Different passphrase should produce different key"
        );
    }
}
