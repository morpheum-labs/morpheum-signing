//! Wallet Adapter trait — Adapter Pattern for injected and external wallets.
//!
//! This trait adapts browser-injected wallets (MetaMask, Phantom, etc.), hardware wallets,
//! and any external signing source into a uniform async interface that `TxBuilder` can use.
//! It is deliberately separate from `Signer` to maintain Interface Segregation and support
//! wallets that cannot directly implement `Signer` (e.g. JavaScript interop in WASM).

use async_trait::async_trait;

use crate::{
    error::SigningError,
    proto::SignDoc,
    types::{Address, Signature, WalletType},
};

/// Adapter for external/injected wallets (MetaMask, Phantom, Taproot, Ledger, etc.).
///
/// **Design Pattern**: Adapter (GoF) — converts the interface of an external wallet
/// into the expected interface for the signing pipeline.
///
/// **Why separate from `Signer`?**
/// - Injected browser wallets have different lifetime/error semantics.
/// - Allows `TxBuilder` to work uniformly with both local keys and injected wallets.
/// - Perfect for WASM/browser environments where signing happens in JavaScript.
#[async_trait]
pub trait WalletAdapter: Send + Sync + 'static {
    /// Requests a raw signature from the external wallet for the canonical `SignDoc`.
    ///
    /// The adapter is responsible for:
    /// - Converting `SignDoc` to the wallet's expected format (EIP-712, Solana message, etc.)
    /// - Calling the wallet's signing API
    /// - Returning the raw signature bytes
    async fn request_signature(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError>;

    /// Returns the type of wallet (used by `TxBuilder` for nonce strategy and mapping).
    fn wallet_type(&self) -> WalletType;

    /// Returns the external address as presented by the wallet.
    ///
    /// This is used by `AddressMapper` to derive the canonical Morpheum `AccountId`.
    fn external_address(&self) -> &Address;

    /// Optional human-readable name of the wallet (e.g. "MetaMask", "Phantom").
    /// Default implementation provided for DRYness.
    fn name(&self) -> &'static str {
        match self.wallet_type() {
            WalletType::Evm => "EVM Wallet (MetaMask/Ledger)",
            WalletType::Solana => "Solana Wallet (Phantom)",
            WalletType::Bitcoin => "Bitcoin Wallet (Taproot)",
            WalletType::Agent => "Agent Wallet (TradingKey)",
            WalletType::Native => "Native Morpheum Keypair",
            WalletType::Hardware => "Hardware Wallet",
        }
    }
}

/// Convenience type alias for dynamic dispatch (used in `TxBuilder` and WASM contexts).
pub type BoxedWalletAdapter = Box<dyn WalletAdapter>;

/// Extension trait for additional convenience methods.
/// Keeps the main `WalletAdapter` trait minimal (Interface Segregation Principle).
#[async_trait]
pub trait WalletAdapterExt: WalletAdapter {
    /// Convenience: requests signature and returns `Signature` wrapper.
    async fn request_signature_wrapped(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
        self.request_signature(sign_doc).await
    }
}

// Blanket implementation for all `WalletAdapter` implementors (DRY)
#[async_trait]
impl<T: WalletAdapter + ?Sized> WalletAdapterExt for T {}