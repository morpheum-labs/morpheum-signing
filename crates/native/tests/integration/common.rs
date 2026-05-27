//! Shared test utilities for the Morpheum Signing SDK test suite.
//!
//! This module provides deterministic test data and reusable helpers to ensure
//! consistency, reproducibility, and maintainability across all integration tests.

use async_trait::async_trait;
use morpheum_signing_core::{
    error::SigningError, nonce::NonceProvider, prelude::*, proto::tx::v1::Nonce,
};

/// Deterministic nonce provider used throughout tests.
/// Returns fixed values to make tests reproducible and easy to reason about.
#[derive(Debug, Clone, Copy)]
pub struct TestNonceProvider {
    pub monotonic: u64,
}

#[async_trait]
impl NonceProvider for TestNonceProvider {
    async fn next_nonce(&self, _account_id: &AccountId) -> Result<Nonce, SigningError> {
        Ok(Nonce {
            monotonic: self.monotonic,
            ts_ms: 1_700_000_000, // Fixed timestamp for deterministic behavior
            sub: 0,
        })
    }

    fn strategy_name(&self) -> &'static str {
        "test_dummy_nonce_provider"
    }
}

/// Standard test seed used across all tests for deterministic signing behavior.
///
/// **Warning**: Never use this seed in production code.
pub const TEST_SEED: [u8; 32] = [42u8; 32];

/// Returns a canonical test `AccountId` used across the test suite.
#[must_use]
pub fn test_account_id() -> AccountId {
    AccountId([0x11u8; 32])
}

/// Returns the current Unix timestamp in seconds (utility for claim tests).
#[must_use]
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_700_000_000)
}

/// Creates a valid test `TradingKeyClaim` for agent-related tests.
///
/// This claim includes a reasonable expiry (24 hours from now) and a nonce
/// sub-range suitable for testing agent delegation and parallelism.
#[must_use]
pub fn test_trading_key_claim() -> TradingKeyClaim {
    let now = now_secs();

    VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1 << 0) // TRADE permission
        .max_daily_usd(1_000_000)
        .expiry(now + 86_400) // 24 hours from now
        .nonce_sub_range(1000, 2000)
        .signature(Signature::Ed25519([1u8; 64])) // non-zero dummy signature for tests
        .build(now)
        .expect("Failed to build test TradingKeyClaim — this should never happen")
}
