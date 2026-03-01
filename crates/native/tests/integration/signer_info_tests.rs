//! Dynamic `SignerInfo` tests — verifies each signer produces correct
//! `public_key_proto()` and `sign_mode()` for its wallet type.
//!
//! This directly addresses the audit's **Critical Issue #1**: hardcoded ed25519.

use morpheum_signing_core::{
    prelude::*,
    signer::Signer,
};
use morpheum_signing_native::prelude::*;

const SEED_A: [u8; 32] = [1u8; 32];
const SEED_B: [u8; 32] = [2u8; 32];

// ==================== NATIVE SIGNER ====================

#[test]
fn test_native_signer_wallet_type() {
    let signer = NativeSigner::from_seed(&SEED_A);
    assert_eq!(signer.wallet_type(), WalletType::Native);
}

#[test]
fn test_native_signer_sign_mode() {
    let signer = NativeSigner::from_seed(&SEED_A);
    assert_eq!(signer.sign_mode(), SignMode::Ed25519);
}

#[test]
fn test_native_signer_public_key_is_ed25519() {
    let signer = NativeSigner::from_seed(&SEED_A);
    match signer.public_key() {
        PublicKey::Ed25519(bytes) => assert_eq!(bytes.len(), 32),
        other => panic!("Expected Ed25519 pubkey, got: {other:?}"),
    }
}

#[test]
fn test_native_signer_public_key_proto_type_url() {
    let signer = NativeSigner::from_seed(&SEED_A);
    let proto = signer.public_key_proto();
    assert_eq!(proto.type_url, "/cosmos.crypto.ed25519.PubKey");
    assert!(!proto.value.is_empty(), "Proto value should contain key bytes");
}

#[test]
fn test_native_signer_different_seeds_different_keys() {
    let s1 = NativeSigner::from_seed(&SEED_A);
    let s2 = NativeSigner::from_seed(&SEED_B);
    assert_ne!(s1.public_key(), s2.public_key());
    assert_ne!(s1.account_id(), s2.account_id());
}

// ==================== AGENT SIGNER ====================

#[test]
fn test_agent_signer_wallet_type() {
    let signer = AgentSigner::new(&SEED_A, AccountId([0x11; 32]), None);
    assert_eq!(signer.wallet_type(), WalletType::Agent);
}

#[test]
fn test_agent_signer_sign_mode() {
    let signer = AgentSigner::new(&SEED_A, AccountId([0x11; 32]), None);
    assert_eq!(signer.sign_mode(), SignMode::Ed25519);
}

#[test]
fn test_agent_signer_public_key_proto() {
    let signer = AgentSigner::new(&SEED_A, AccountId([0x11; 32]), None);
    let proto = signer.public_key_proto();
    assert_eq!(proto.type_url, "/cosmos.crypto.ed25519.PubKey");
    assert!(!proto.value.is_empty());
}

#[test]
fn test_agent_signer_uses_provided_account_id() {
    let custom_id = AccountId([0xAA; 32]);
    let signer = AgentSigner::new(&SEED_A, custom_id.clone(), None);
    assert_eq!(signer.account_id(), custom_id, "Agent should use provided AccountId");
}

// ==================== EVM SIGNER ====================

#[test]
fn test_evm_signer_wallet_type() {
    let signer = EvmSigner::from_seed(&SEED_A);
    assert_eq!(signer.wallet_type(), WalletType::Evm);
}

#[test]
fn test_evm_signer_sign_mode() {
    let signer = EvmSigner::from_seed(&SEED_A);
    assert_eq!(signer.sign_mode(), SignMode::Secp256k1);
}

#[test]
fn test_evm_signer_public_key_is_secp256k1() {
    let signer = EvmSigner::from_seed(&SEED_A);
    match signer.public_key() {
        PublicKey::Secp256k1(bytes) => assert_eq!(bytes.len(), 33, "Compressed secp256k1 key is 33 bytes"),
        other => panic!("Expected Secp256k1 pubkey, got: {other:?}"),
    }
}

#[test]
fn test_evm_signer_public_key_proto_type_url() {
    let signer = EvmSigner::from_seed(&SEED_A);
    let proto = signer.public_key_proto();
    assert_eq!(proto.type_url, "/cosmos.crypto.secp256k1.PubKey");
    assert_eq!(proto.value.len(), 33);
}

// ==================== SOLANA SIGNER ====================

#[test]
fn test_solana_signer_wallet_type() {
    let signer = SolanaSigner::from_seed(&SEED_A);
    assert_eq!(signer.wallet_type(), WalletType::Solana);
}

#[test]
fn test_solana_signer_sign_mode() {
    let signer = SolanaSigner::from_seed(&SEED_A);
    assert_eq!(signer.sign_mode(), SignMode::Ed25519);
}

#[test]
fn test_solana_signer_public_key_is_ed25519() {
    let signer = SolanaSigner::from_seed(&SEED_A);
    match signer.public_key() {
        PublicKey::Ed25519(bytes) => assert_eq!(bytes.len(), 32),
        other => panic!("Expected Ed25519 pubkey, got: {other:?}"),
    }
}

#[test]
fn test_solana_signer_public_key_proto_type_url() {
    let signer = SolanaSigner::from_seed(&SEED_A);
    let proto = signer.public_key_proto();
    assert_eq!(proto.type_url, "/cosmos.crypto.ed25519.PubKey");
    assert_eq!(proto.value.len(), 32);
}

// ==================== BITCOIN SIGNER ====================

#[test]
fn test_bitcoin_signer_wallet_type() {
    let signer = BitcoinSigner::from_seed(&SEED_A);
    assert_eq!(signer.wallet_type(), WalletType::Bitcoin);
}

#[test]
fn test_bitcoin_signer_sign_mode() {
    let signer = BitcoinSigner::from_seed(&SEED_A);
    assert_eq!(signer.sign_mode(), SignMode::SchnorrAggregate);
}

#[test]
fn test_bitcoin_signer_public_key_is_schnorr() {
    let signer = BitcoinSigner::from_seed(&SEED_A);
    match signer.public_key() {
        PublicKey::Schnorr(bytes) => assert_eq!(bytes.len(), 32, "X-only pubkey is 32 bytes"),
        other => panic!("Expected Schnorr pubkey, got: {other:?}"),
    }
}

#[test]
fn test_bitcoin_signer_public_key_proto_type_url() {
    let signer = BitcoinSigner::from_seed(&SEED_A);
    let proto = signer.public_key_proto();
    assert_eq!(proto.type_url, "/morpheum.crypto.schnorr.PubKey");
    assert_eq!(proto.value.len(), 32);
}

// ==================== CROSS-SIGNER CONSISTENCY ====================

#[test]
fn test_all_signers_return_non_empty_proto_value() {
    let native = NativeSigner::from_seed(&SEED_A);
    let agent = AgentSigner::new(&SEED_A, AccountId([0x11; 32]), None);
    let evm = EvmSigner::from_seed(&SEED_A);
    let sol = SolanaSigner::from_seed(&SEED_A);
    let btc = BitcoinSigner::from_seed(&SEED_A);

    for (name, signer) in [
        ("native", &native as &dyn Signer),
        ("agent", &agent as &dyn Signer),
        ("evm", &evm as &dyn Signer),
        ("solana", &sol as &dyn Signer),
        ("bitcoin", &btc as &dyn Signer),
    ] {
        let proto = signer.public_key_proto();
        assert!(!proto.type_url.is_empty(), "{name}: type_url should not be empty");
        assert!(!proto.value.is_empty(), "{name}: proto value should not be empty");
    }
}

#[test]
fn test_account_id_derived_from_public_key() {
    let signer = NativeSigner::from_seed(&SEED_A);
    let expected_id = signer.public_key().to_account_id();
    assert_eq!(signer.account_id(), expected_id);
}
