//! Shared test helpers and constants for the Morpheum Signing SDK tests.
//!
//! This module provides reusable utilities across all integration tests,
//! ensuring consistency, maintainability, and production-grade quality.
//! It includes test nonce providers, deterministic seeds, and helper builders.

use async_trait::async_trait;
use morpheum_signing_core::{
    error::SigningError,
    nonce::NonceProvider,
    prelude::*,
    proto::tx::v1::Nonce,
};
use morpheum_signing_native::prelude::*;

/// A deterministic nonce provider for tests.
/// Always returns a predictable nonce, making tests reproducible.
#[derive(Debug, Clone, Copy)]
pub struct TestNonceProvider {
    pub monotonic: u64,
}

#[async_trait]
impl NonceProvider for TestNonceProvider {
    async fn next_nonce(&self, _account_id: &AccountId) -> Result<Nonce, SigningError> {
        Ok(Nonce {
            monotonic: self.monotonic,
            ts_ms: 1_700_000_000, // Fixed timestamp for deterministic tests
            sub: 0,
        })
    }

    fn strategy_name(&self) -> &'static str {
        "test_dummy_nonce_provider"
    }
}

/// Standard test seed used across all tests for deterministic signing.
/// In production code, never hardcode seeds like this.
pub const TEST_SEED: [u8; 32] = [42u8; 32];

/// Returns a canonical test AccountId for use in tests.
#[must_use]
pub fn test_account_id() -> AccountId {
    AccountId([0x11u8; 32])
}

/// Creates a valid test `TradingKeyClaim` for agent-related tests.
///
/// This claim has a 24-hour expiry and a reasonable nonce sub-range.
#[must_use]
pub fn test_trading_key_claim() -> TradingKeyClaim {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_700_000_000);

    VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1 << 0) // TRADE permission
        .max_daily_usd(1_000_000)
        .expiry(now_secs + 86_400) // 24 hours
        .nonce_sub_range(1000, 2000)
        .signature(Signature(vec![0u8; 64])) // dummy signature for tests
        .build()
        .expect("Failed to build test TradingKeyClaim")
}