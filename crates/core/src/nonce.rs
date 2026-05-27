//! Nonce Provider trait — Strategy Pattern for different nonce fetching strategies.
//!
//! This trait abstracts how nonces are obtained for both humans and agents:
//! - Sentry nodes (sequential for humans / `MetaMask` compatibility)
//! - `AgentPortal` (monotonic + `ts_ms` + sub-range for AI agents with `TradingKey` VC)
//!
//! Concrete implementations live in the `native` crate.
//! This core trait is `no_std` compatible and object-safe.

use async_trait::async_trait;

use crate::{error::SigningError, proto::tx::v1::Nonce, types::AccountId};

/// Strategy for obtaining the next nonce for an account.
///
/// **Design Pattern**: Strategy (`GoF`) — allows `TxBuilder` to swap between
/// different nonce fetching strategies (Sentry vs `AgentPortal`) without knowing
/// the implementation details.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait NonceProvider: Send + Sync + 'static {
    /// Returns the next valid `Nonce` for the given `AccountId`.
    ///
    /// The returned `Nonce` is the exact protobuf structure expected by Morpheum:
    /// - `monotonic`: soft ordering (>= `last_monotonic`)
    /// - `ts_ms`: timestamp for replay protection
    /// - `sub`: sub-stream identifier for `TradingKey` parallelism
    async fn next_nonce(&self, account_id: &AccountId) -> Result<Nonce, SigningError>;

    /// Returns a human-readable name of the strategy (for logging/debugging).
    fn strategy_name(&self) -> &'static str {
        "unknown_nonce_strategy"
    }
}

/// Convenience type alias for dynamic dispatch (used in `TxBuilder`).
pub type BoxedNonceProvider = Box<dyn NonceProvider>;
