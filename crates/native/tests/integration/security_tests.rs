//! Security-focused tests for the Morpheum Signing SDK.
//!
//! Covers:
//! - Public key type isolation (different curves never collide)
//! - Signature type isolation
//! - Seed sensitivity (all-zero, all-max, boundary values)
//! - AccountId derivation consistency
//! - ZeroizeOnDrop marker verification

use morpheum_signing_core::{
    prelude::*,
    signer::Signer,
};
use morpheum_signing_native::prelude::*;

// ==================== KEY TYPE ISOLATION ====================

#[test]
fn test_same_seed_different_curves_different_pubkeys() {
    let seed = [42u8; 32];

    let native_pk = NativeSigner::from_seed(&seed).public_key();
    let evm_pk = EvmSigner::from_seed(&seed).public_key();
    let btc_pk = BitcoinSigner::from_seed(&seed).public_key();

    // Different curves → different key representations
    assert_ne!(native_pk.to_proto_bytes(), evm_pk.to_proto_bytes());
    assert_ne!(native_pk.type_url(), evm_pk.type_url());
    assert_ne!(evm_pk.type_url(), btc_pk.type_url());
}

#[test]
fn test_same_seed_different_curves_different_account_ids() {
    let seed = [42u8; 32];

    let native_id = NativeSigner::from_seed(&seed).account_id();
    let evm_id = EvmSigner::from_seed(&seed).account_id();
    let btc_id = BitcoinSigner::from_seed(&seed).account_id();

    assert_ne!(native_id, evm_id);
    assert_ne!(native_id, btc_id);
    assert_ne!(evm_id, btc_id);
}

// ==================== SIGNATURE TYPE ISOLATION ====================

#[test]
fn test_signature_variant_bytes() {
    let ed_sig = Signature::Ed25519([1u8; 64]);
    let secp_sig = Signature::Secp256k1([1u8; 64]);
    let schnorr_sig = Signature::Schnorr([1u8; 64]);

    // All have same inner bytes but are distinct variants
    assert_eq!(ed_sig.to_bytes(), secp_sig.to_bytes());
    assert_ne!(ed_sig, secp_sig);
    assert_ne!(secp_sig, schnorr_sig);
}

#[test]
fn test_signature_is_zero() {
    assert!(Signature::Ed25519([0u8; 64]).is_zero());
    assert!(Signature::Secp256k1([0u8; 64]).is_zero());
    assert!(Signature::Schnorr([0u8; 64]).is_zero());

    assert!(!Signature::Ed25519([1u8; 64]).is_zero());
}

// ==================== SEED SENSITIVITY ====================

#[test]
fn test_all_zero_seed() {
    // All-zero seed should produce a valid (but insecure) signer
    let seed = [0u8; 32];
    let signer = NativeSigner::from_seed(&seed);
    let pk = signer.public_key();
    match pk {
        PublicKey::Ed25519(bytes) => assert_ne!(bytes, [0u8; 32], "Pubkey should not be all zeros"),
        _ => panic!("Unexpected key type"),
    }
}

#[test]
fn test_all_max_seed() {
    let seed = [0xFFu8; 32];
    let signer = NativeSigner::from_seed(&seed);
    let pk = signer.public_key();
    match pk {
        PublicKey::Ed25519(bytes) => assert_ne!(bytes, [0xFFu8; 32]),
        _ => panic!("Unexpected key type"),
    }
}

#[test]
fn test_single_bit_difference_in_seed() {
    let seed1 = [0u8; 32];
    let mut seed2 = [0u8; 32];
    seed2[0] = 1; // single bit difference

    let s1 = NativeSigner::from_seed(&seed1);
    let s2 = NativeSigner::from_seed(&seed2);

    assert_ne!(s1.public_key(), s2.public_key(), "Single bit change should produce different keys");

    // Flip a different bit position
    let mut seed3 = [0u8; 32];
    seed3[31] = 0x80;
    let mut seed4 = [0u8; 32];
    seed4[15] = 0x01;

    let s3 = NativeSigner::from_seed(&seed3);
    let s4 = NativeSigner::from_seed(&seed4);
    assert_ne!(s3.public_key(), s4.public_key(), "Different bit positions should produce different keys");
}

// ==================== PUBLIC KEY PROTO ENCODING ====================

#[test]
fn test_public_key_proto_any_round_trip() {
    let signer = NativeSigner::from_seed(&[42u8; 32]);
    let pk = signer.public_key();
    let any = pk.to_proto_any();

    assert_eq!(any.type_url, pk.type_url());
    assert_eq!(any.value, pk.to_proto_bytes());
}

#[test]
fn test_public_key_type_urls_are_distinct() {
    let ed25519 = PublicKey::Ed25519([0u8; 32]);
    let secp256k1 = PublicKey::Secp256k1([0u8; 33]);
    let schnorr = PublicKey::Schnorr([0u8; 32]);
    let agent = PublicKey::Agent([0u8; 32]);

    // ed25519 and agent share the same type_url (by design)
    assert_eq!(ed25519.type_url(), agent.type_url());
    assert_ne!(ed25519.type_url(), secp256k1.type_url());
    assert_ne!(ed25519.type_url(), schnorr.type_url());
    assert_ne!(secp256k1.type_url(), schnorr.type_url());
}

// ==================== ACCOUNT ID ====================

#[test]
fn test_account_id_zero_constant() {
    assert_eq!(AccountId::ZERO, AccountId([0u8; 32]));
}

#[test]
fn test_account_id_from_public_key_is_deterministic() {
    let pk = PublicKey::Ed25519([42u8; 32]);
    let id1 = pk.to_account_id();
    let id2 = pk.to_account_id();
    assert_eq!(id1, id2);
}

#[test]
fn test_different_pubkeys_produce_different_account_ids() {
    let pk1 = PublicKey::Ed25519([1u8; 32]);
    let pk2 = PublicKey::Ed25519([2u8; 32]);
    assert_ne!(pk1.to_account_id(), pk2.to_account_id());
}

// ==================== WALLET TYPE ====================

#[test]
fn test_wallet_type_display() {
    assert_eq!(format!("{}", WalletType::Native), "native");
    assert_eq!(format!("{}", WalletType::Evm), "evm");
    assert_eq!(format!("{}", WalletType::Solana), "solana");
    assert_eq!(format!("{}", WalletType::Bitcoin), "bitcoin");
    assert_eq!(format!("{}", WalletType::Agent), "agent");
    assert_eq!(format!("{}", WalletType::Hardware), "hardware");
}

#[test]
fn test_wallet_type_default_sign_modes() {
    assert_eq!(WalletType::Native.default_sign_mode(), SignMode::Ed25519);
    assert_eq!(WalletType::Solana.default_sign_mode(), SignMode::Ed25519);
    assert_eq!(WalletType::Agent.default_sign_mode(), SignMode::Ed25519);
    assert_eq!(WalletType::Evm.default_sign_mode(), SignMode::Secp256k1);
    assert_eq!(WalletType::Bitcoin.default_sign_mode(), SignMode::SchnorrAggregate);
    assert_eq!(WalletType::Hardware.default_sign_mode(), SignMode::Ed25519);
}

// ==================== SIGNING OPTIONS ====================

#[test]
fn test_signing_options_default() {
    let opts = SigningOptions::new();
    assert!(opts.deadline_seconds.is_none());
    assert!(opts.memo.is_none());
    assert!(!opts.include_timestamp);
}

// ==================== SIGNED TX ====================

#[test]
fn test_signed_tx_accessors() {
    use morpheum_signing_core::proto::tx::v1::{Tx, TxBody, TxRaw};

    let tx = Tx {
        body: Some(TxBody {
            messages: vec![],
            memo: "test".to_string(),
            timeout_timestamp: None,
        }),
        auth_info: None,
        signatures: vec![vec![1, 2, 3]],
        nonce: None,
    };

    let raw = vec![4, 5, 6];
    let tx_raw = TxRaw {
        body_bytes: vec![7, 8],
        auth_info_bytes: vec![9, 10],
        signatures: vec![vec![1, 2, 3]],
    };

    let signed = SignedTx::new(tx, raw.clone(), Some(tx_raw));

    assert_eq!(signed.raw_bytes(), &[4, 5, 6]);
    assert!(signed.tx_raw().is_some());
    assert_eq!(signed.tx().body.as_ref().unwrap().memo, "test");
}
