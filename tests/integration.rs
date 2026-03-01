//! Main integration test suite for the Morpheum Signing SDK.
//!
//! This is the single entry point for running all integration tests.
//! It verifies the full functionality of the library across all supported flows:
//! - Native signer (Morpheum ed25519)
//! - Agent signer (TradingKey + VC claims)
//! - Multi-chain address mapping
//! - Error handling and edge cases
//!
//! Run with: `cargo test --test integration`

mod common;
mod native_flow;
mod agent_flow;
mod multi_chain;
mod error_cases;

#[tokio::test]
async fn full_integration_test_suite() {
    println!("🚀 Starting Morpheum Signing SDK Integration Test Suite");
    println!("=====================================================");

    // Run all test modules in a logical order
    println!("\n📋 Running Native signer flow...");
    native_flow::test_native_signing_flow().await;

    println!("\n📋 Running Agent signer flow...");
    agent_flow::test_agent_signing_flow().await;

    println!("\n📋 Running Multi-chain address mapping...");
    multi_chain::test_multi_chain_address_mapping().await;

    println!("\n📋 Running Error cases and negative paths...");
    error_cases::test_error_cases().await;

    println!("\n🎉 All integration tests passed successfully!");
    println!("    Native signer  ✓");
    println!("    Agent signer   ✓");
    println!("    Multi-chain    ✓");
    println!("    Error handling ✓");
    println!("\nThe Morpheum Signing SDK is ready for production use.");
}