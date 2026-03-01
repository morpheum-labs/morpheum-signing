//! `AddressMapper` trait — Strategy Pattern for mapping external chain addresses
//! to Morpheum canonical `AccountId`.
//!
//! This abstraction allows `TxBuilder` to work uniformly with any address format
//! (EVM, Solana, Bitcoin Taproot, Native, Agent DID, etc.) without coupling
//! to concrete parsing or hashing logic.

use crate::{
    error::SigningError,
    types::{AccountId, Address},
};

/// Strategy for mapping an external chain address to the canonical Morpheum `AccountId`.
///
/// **Design Pattern**: Strategy (`GoF`) — allows easy extension for new chains
/// or custom mapping rules (e.g. ENS resolution, custom prefixes) without
/// modifying `TxBuilder` or existing code.
///
/// This trait is synchronous because address mapping is a pure, fast computation.
pub trait AddressMapper: Send + Sync + 'static {
    /// Maps an external `Address` to the canonical Morpheum `AccountId` (blake3 hash).
    ///
    /// # Errors
    ///
    /// Returns `SigningError::AddressMapping` on failure (invalid format, unsupported chain, etc.).
    fn to_account_id(&self, address: &Address) -> Result<AccountId, SigningError>;

    /// Returns a human-readable name of the mapper (for logging/debugging).
    fn name(&self) -> &'static str {
        "default_address_mapper"
    }
}

/// Convenience type alias for dynamic dispatch (used in `TxBuilder`).
pub type BoxedAddressMapper = Box<dyn AddressMapper>;

/// Default implementation that delegates to the built-in `Address::to_account_id()` method.
///
/// This is the recommended mapper for standard use cases and is used by default in `TxBuilder`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultAddressMapper;

impl DefaultAddressMapper {
    /// Creates a new default mapper.
    #[must_use] 
    pub const fn new() -> Self {
        Self
    }
}

impl AddressMapper for DefaultAddressMapper {
    fn to_account_id(&self, address: &Address) -> Result<AccountId, SigningError> {
        Ok(address.to_account_id())
    }

    fn name(&self) -> &'static str {
        "default_blake3_mapper"
    }
}

/// Extension trait to keep the main trait minimal (Interface Segregation Principle).
pub trait AddressMapperExt: AddressMapper {
    /// Convenience method with clearer name for chaining.
    ///
    /// # Errors
    ///
    /// Delegates to [`AddressMapper::to_account_id`]; returns the same errors.
    fn map_to_account_id(&self, address: &Address) -> Result<AccountId, SigningError> {
        self.to_account_id(address)
    }
}

// Blanket implementation for DRYness
impl<T: AddressMapper + ?Sized> AddressMapperExt for T {}