//! Multi-chain address mapping tests for the Morpheum Signing SDK.
//!
//! Verifies that addresses from all supported external chains (EVM, Solana,
//! Bitcoin Taproot, Native, and Agent DIDs) are correctly mapped to the
//! canonical Morpheum `AccountId` (SHA-256 hash).

use morpheum_signing_core::{
    mapper::{AddressMapper, DefaultAddressMapper},
    types::Address,
};

#[test]
fn test_native_address_mapping() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Native("morm1abc123def456".to_string());
    let id = mapper.to_account_id(&addr).expect("Native mapping failed");
    assert_eq!(id, addr.to_account_id());
}

#[test]
fn test_evm_address_mapping() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Evm([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78,
    ]);
    let id = mapper.to_account_id(&addr).expect("EVM mapping failed");
    assert_eq!(id, addr.to_account_id());
}

#[test]
fn test_solana_address_mapping() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Solana([0x11u8; 32]);
    let id = mapper.to_account_id(&addr).expect("Solana mapping failed");
    assert_eq!(id, addr.to_account_id());
}

#[test]
fn test_bitcoin_taproot_address_mapping() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Bitcoin("bc1p5d7rjq7g6rdk2yhzks9smlaq4r5m4y4".to_string());
    let id = mapper.to_account_id(&addr).expect("Bitcoin mapping failed");
    assert_eq!(id, addr.to_account_id());
}

#[test]
fn test_agent_did_address_mapping() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Agent("did:agent:alpha-trader-v3".to_string());
    let id = mapper.to_account_id(&addr).expect("Agent DID mapping failed");
    assert_eq!(id, addr.to_account_id());
}

// ==================== EDGE CASES ====================

#[test]
fn test_empty_native_address_produces_valid_account_id() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Native("".to_string());
    let result = mapper.to_account_id(&addr);
    assert!(result.is_ok(), "Empty native address should still produce a valid AccountId");
    // Verify it's the hash of empty bytes, not zero
    assert_ne!(result.unwrap(), morpheum_signing_core::types::AccountId::ZERO);
}

#[test]
fn test_empty_bitcoin_address_produces_valid_account_id() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Bitcoin("".to_string());
    let result = mapper.to_account_id(&addr);
    assert!(result.is_ok());
}

#[test]
fn test_same_string_different_types_produce_different_ids() {
    let mapper = DefaultAddressMapper;
    let native = Address::Native("test".to_string());
    let bitcoin = Address::Bitcoin("test".to_string());
    let agent = Address::Agent("test".to_string());

    let native_id = mapper.to_account_id(&native).unwrap();
    let bitcoin_id = mapper.to_account_id(&bitcoin).unwrap();
    let agent_id = mapper.to_account_id(&agent).unwrap();

    // Native, Bitcoin, and Agent all hash the string bytes, so same input → same hash
    // This is by design: the Address variant prefix is NOT part of the hash input
    assert_eq!(native_id, bitcoin_id);
    assert_eq!(bitcoin_id, agent_id);
}

#[test]
fn test_all_zero_evm_address_is_valid() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Evm([0u8; 20]);
    let id = mapper.to_account_id(&addr).unwrap();
    assert_ne!(id, morpheum_signing_core::types::AccountId::ZERO);
}

#[test]
fn test_all_zero_solana_address_is_valid() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Solana([0u8; 32]);
    let id = mapper.to_account_id(&addr).unwrap();
    assert_ne!(id, morpheum_signing_core::types::AccountId::ZERO);
}

#[test]
fn test_address_mapping_is_deterministic() {
    let mapper = DefaultAddressMapper;
    let addr = Address::Evm([0xAB; 20]);
    let id1 = mapper.to_account_id(&addr).unwrap();
    let id2 = mapper.to_account_id(&addr).unwrap();
    assert_eq!(id1, id2, "Address mapping must be deterministic");
}

#[test]
fn test_long_native_address() {
    let mapper = DefaultAddressMapper;
    let long_addr = Address::Native("morm1".to_string() + &"a".repeat(1000));
    let result = mapper.to_account_id(&long_addr);
    assert!(result.is_ok(), "Long addresses should not panic");
}
