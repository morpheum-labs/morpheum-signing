//! Nonce provider implementations for native environments.
//!
//! This module provides concrete implementations of the `NonceProvider` trait
//! from `morpheum-signing-core`:
//!
//! - **SentryNonceProvider** — Sequential nonce fetching from Sentry nodes.
//!   Used for human / MetaMask-style flows (compatible with sequential nonce expectations).
//! - **PortalNonceProvider** — Hot-path monotonic nonce generation for AgentPortal.
//!   Optimized for AI agents with TradingKey VC delegation and sub-range isolation.
//!
//! Both providers require the `http` feature (enabled by default in the `native` crate).
//!
//! **Design**:
//! - Follows the **Strategy Pattern** (GoF) — allows `TxBuilder` to swap nonce strategies
//!   without knowing implementation details.
//! - Clear separation between human (sequential) and agent (monotonic + sub-range) flows.
//! - Re-exports are structured identically to `signers/mod.rs` and `adapters/mod.rs`
//!   for consistency across the entire native crate.
//!
//! Usage:
//! ```rust,ignore
//! use morpheum_signing_native::providers::Sentry;
//!
//! let builder = TxBuilder::human(signer)
//!     .with_nonce_provider(Sentry::local());
//! ```

// Module declarations
pub mod sentry;
pub mod portal;

// Public re-exports (feature-gated to match the providers' own requirements)
#[cfg(feature = "http")]
pub use sentry::SentryNonceProvider;
#[cfg(feature = "http")]
pub use portal::PortalNonceProvider;

// Convenience type aliases (matching the exact style used in signers/ and adapters/)
#[cfg(feature = "http")]
pub type Sentry = SentryNonceProvider;
#[cfg(feature = "http")]
pub type Portal = PortalNonceProvider;

/// Re-export the core trait for ergonomic imports when implementing custom providers.
pub use morpheum_signing_core::nonce::NonceProvider;