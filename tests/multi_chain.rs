//! Multi-chain address mapping tests for the Morpheum Signing SDK.
//!
//! Verifies that addresses from all supported external chains (EVM, Solana,
//! Bitcoin Taproot, Native, and Agent DIDs) are correctly mapped to the
//! canonical Morpheum `AccountId` (blake3 hash).
//!
//! This ensures full interoperability with MetaMask, Phantom, Unisat,
//! Leather, and native Morpheum accounts.

use super::common::*;
use morpheum_signing_core::{
    mapper::{AddressMapper, DefaultAddressMapper},
    types::Address,
};

#[tokio::test]
pub async fn test_multi_chain_address_mapping() {
    println!("🧪 Running Multi-Chain Address Mapping Test Suite...");

    let mapper = DefaultAddressMapper;

    // ==================== NATIVE (Morpheum native) ====================
    println!("   • Testing Native Morpheum address...");
    let native_addr = Address::Native("morm1abc123def456".to_string());
    let native_id = mapper.to_account_id(&native_addr).expect("Native address mapping failed");
    assert_eq!(native_id, native_addr.to_account_id(), "Native address mapping mismatch");

    // ==================== EVM (MetaMask, Rabby, Ledger) ====================
    println!("   • Testing EVM address...");
    let evm_addr = Address::Evm([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34,
    ]);
    let evm_id = mapper.to_account_id(&evm_addr).expect("EVM address mapping failed");
    assert_eq!(evm_id, evm_addr.to_account_id(), "EVM address mapping mismatch");

    // ==================== SOLANA (Phantom, Solflare) ====================
    println!("   • Testing Solana address...");
    let sol_addr = Address::Solana([0x11u8; 32]);
    let sol_id = mapper.to_account_id(&sol_addr).expect("Solana address mapping failed");
    assert_eq!(sol_id, sol_addr.to_account_id(), "Solana address mapping mismatch");

    // ==================== BITCOIN TAPROOT ====================
    println!("   • Testing Bitcoin Taproot address...");
    let btc_addr = Address::Bitcoin("bc1p5d7rjq7g6rdk2yhzks9smlaq4r5m4y4".to_string());
    let btc_id = mapper.to_account_id(&btc_addr).expect("Bitcoin Taproot mapping failed");
    assert_eq!(btc_id, btc_addr.to_account_id(), "Bitcoin Taproot mapping mismatch");

    // ==================== AGENT DID ====================
    println!("   • Testing Agent DID...");
    let agent_addr = Address::Agent("did:agent:alpha-trader-v3".to_string());
    let agent_id = mapper.to_account_id(&agent_addr).expect("Agent DID mapping failed");
    assert_eq!(agent_id, agent_addr.to_account_id(), "Agent DID mapping mismatch");

    // ==================== EDGE CASE: EMPTY ADDRESS ====================
    println!("   • Testing empty native address edge case...");
    let empty_addr = Address::Native("".to_string());
    let result = mapper.to_account_id(&empty_addr);
    assert!(result.is_ok(), "Empty native address should still produce a valid AccountId");

    println!("✅ All multi-chain address mapping tests passed successfully");
}