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
// Needs two features, and says so rather than relying on the caller
// reaching for `--all-features`:
//
// - `cryptogram`, for `morpheum_signing_core::cryptogram_crypto`.
// - `verifier`, for `verify_signed_tx` — core gates its `verifier`
//   module on `morpheum-signing-core/full-crypto`, which this crate
//   forwards only through its own `verifier` feature. Note `full` does
//   NOT include it despite being commented "Everything", so a default
//   build has the module's other dependencies but not this one.
//
// Declared ungated, it was compiled under every feature set and failed
// to resolve those paths — 34 errors on a plain `cargo test`, which is
// what the `--all-features` note above was working around. Gated, it
// compiles out where its requirements are absent and the rest of the
// suite still runs.
#[cfg(all(feature = "cryptogram", feature = "verifier"))]
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
