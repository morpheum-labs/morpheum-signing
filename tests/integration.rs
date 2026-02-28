//! Main integration test entrypoint.
//! Run with: cargo test --test integration

mod common;
mod human_flow;
mod agent_flow;
mod multi_chain;
mod error_cases;

#[tokio::test]
async fn full_integration_test_suite() {
    println!("🚀 Running Morpheum Signing SDK integration test suite...");

    human_flow::test_human_signing_flow().await;
    agent_flow::test_agent_signing_flow().await;
    multi_chain::test_multi_chain_mapping().await;
    error_cases::test_error_cases().await;

    println!("✅ All integration tests passed!");
}