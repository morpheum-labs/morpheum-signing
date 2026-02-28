//! Shared test helpers and constants for the signing crate tests.

use morpheum_signing_core::{
    prelude::*,
    nonce::NonceProvider,
    types::AccountId,
};
use async_trait::async_trait;

/// Dummy nonce provider for tests (always returns predictable nonce).
#[derive(Debug, Clone)]
pub struct TestNonceProvider {
    pub monotonic: u64,
}

#[async_trait]
impl NonceProvider for TestNonceProvider {
    async fn next_nonce(&self, _account_id: &AccountId) -> Result<Nonce, SigningError> {
        Ok(Nonce {
            monotonic: self.monotonic,
            ts_ms: 1_700_000_000, // fixed timestamp for deterministic tests
            sub: 0,
        })
    }

    fn strategy_name(&self) -> &'static str {
        "test_dummy_nonce_provider"
    }
}

/// Test vector seed for deterministic signing tests.
pub const TEST_SEED: [u8; 32] = [42u8; 32];

/// Test AccountId (blake3 of a known value).
pub fn test_account_id() -> AccountId {
    AccountId([0x11u8; 32])
}

/// Test TradingKeyClaim for agent tests.
pub fn test_trading_key_claim() -> TradingKeyClaim {
    VcClaimBuilder::new()
        .issuer(test_account_id())
        .subject(test_account_id())
        .permissions(1 << 0) // TRADE
        .max_daily_usd(1_000_000)
        .expiry(1_800_000_000)
        .nonce_sub_range(1000, 2000)
        .signature(Signature(vec![0u8; 64])) // dummy signature for test
        .build()
        .unwrap()
}