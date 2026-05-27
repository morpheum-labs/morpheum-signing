//! Main integration test suite for the Morpheum Signing SDK.
//!
//! Single entry point for running all integration tests.
//! Verifies the full functionality across all supported flows:
//! - Native signer (Morpheum ed25519)
//! - Agent signer (TradingKey + VC claims)
//! - Multi-chain address mapping
//! - EVM / Solana / Bitcoin signing flows
//! - Dynamic signer info (audit Critical Issue #1)
//! - TradingKeyClaim verification & encoding (audit Critical Issue #2)
//! - Error handling and edge cases
//! - Security properties
//! - Cross-crate integration (cryptogram ↔ signing)
//!
//! Run with: `cargo test -p morpheum-signing-native --test integration --all-features`

#[path = "integration/agent_flow.rs"]
mod agent_flow;
#[path = "integration/claim_tests.rs"]
mod claim_tests;
#[path = "integration/common.rs"]
mod common;
#[path = "integration/cross_crate_signing.rs"]
mod cross_crate_signing;
#[path = "integration/error_cases.rs"]
mod error_cases;
#[path = "integration/multi_chain.rs"]
mod multi_chain;
#[path = "integration/native_flow.rs"]
mod native_flow;
#[path = "integration/security_tests.rs"]
mod security_tests;
#[path = "integration/signer_info_tests.rs"]
mod signer_info_tests;
#[path = "integration/signing_flows.rs"]
mod signing_flows;
