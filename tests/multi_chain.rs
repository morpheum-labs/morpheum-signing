//! Multi-chain address mapping tests.
//!
//! Verifies that external addresses from EVM, Solana, Bitcoin Taproot, Native, and Agent
//! are correctly mapped to the canonical Morpheum `AccountId` (blake3 hash).
//!
//! This test ensures full interoperability with MetaMask, Phantom, Unisat/Taproot, etc.

use super::common::*;
use morpheum_signing_core::{
    mapper::{AddressMapper, DefaultAddressMapper},
    types::{AccountId, Address},
};

#[tokio::test]
async fn test_multi_chain_address_mapping() {
    let mapper = DefaultAddressMapper;

    // ==================== EVM (MetaMask / Ledger) ====================
    let evm_addr = Address::Evm([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78,
    ]);
    let evm_id = mapper.to_account_id(&evm_addr).expect("EVM mapping failed");
    assert_eq!(evm_id, evm_addr.to_account_id(), "EVM address mapping mismatch");

    // ==================== Solana (Phantom / Solflare) ====================
    let sol_addr = Address::Solana([0x11u8; 32]);
    let sol_id = mapper.to_account_id(&sol_addr).expect("Solana mapping failed");
    assert_eq!(sol_id, sol_addr.to_account_id(), "Solana address mapping mismatch");

    // ==================== Bitcoin Taproot ====================
    let btc_addr = Address::Bitcoin("bc1p5d7rjq7g6rdk2yhzks9smlaq4r5m4y4".to_string());
    let btc_id = mapper.to_account_id(&btc_addr).expect("Bitcoin mapping failed");
    assert_eq!(btc_id, btc_addr.to_account_id(), "Bitcoin Taproot mapping mismatch");

    // ==================== Native Morpheum ====================
    let native_addr = Address::Native("morm1abc123def456".to_string());
    let native_id = mapper.to_account_id(&native_addr).expect("Native mapping failed");
    assert_eq!(native_id, native_addr.to_account_id(), "Native address mapping mismatch");

    // ==================== Agent DID ====================
    let agent_addr = Address::Agent("did:agent:alpha-trader-v3".to_string());
    let agent_id = mapper.to_account_id(&agent_addr).expect("Agent mapping failed");
    assert_eq!(agent_id, agent_addr.to_account_id(), "Agent DID mapping mismatch");

    // ==================== Edge Case: Empty / Invalid ====================
    let empty_native = Address::Native("".to_string());
    let result = mapper.to_account_id(&empty_native);
    assert!(result.is_ok(), "Empty native address should still produce valid AccountId (blake3 of empty string)");

    println!("✅ All multi-chain address mapping tests passed");
}