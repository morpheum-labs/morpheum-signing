//! `TxBuilder` — Fluent, generic, proto-centric transaction builder.
//!
//! This is the **main public API** of the signing library.
//! It is deliberately **completely generic** — it has no knowledge of any specific
//! module messages (MsgCreateMarketRequest, etc.). Those belong in a higher-level SDK.
//!
//! Design: Builder Pattern + Generics over `Signer` for zero-cost abstraction.

use alloc::vec::Vec;
use core::fmt;

use prost::Message;

use crate::{
    claim::TradingKeyClaim,
    error::SigningError,
    mapper::{AddressMapper, DefaultAddressMapper},
    nonce::{BoxedNonceProvider, NonceProvider},
    proto::tx::v1::{
        self as tx, AuthInfo, ModeInfo, Nonce, SignDoc, SignerInfo, Tx, TxBody, TxRaw,
    },
    signer::Signer,
    types::{SignedTx, SigningOptions},
    wallet_adapter::{BoxedWalletAdapter, WalletAdapter},
};

/// Fluent transaction builder (completely generic).
///
/// Generic over the signer to allow zero-cost monomorphization for local keys
/// while supporting dynamic dispatch for injected wallets.
pub struct TxBuilder<S: Signer> {
    signer: S,
    chain_id: String,
    account_number: Option<u64>,
    memo: Option<String>,
    timeout_timestamp: Option<u64>, // seconds since epoch
    messages: Vec<prost_types::Any>, // ← ONLY generic Any
    signing_options: SigningOptions,
    nonce_provider: Option<BoxedNonceProvider>,
    #[allow(dead_code)]
    address_mapper: Box<dyn AddressMapper>,
    wallet_adapter: Option<BoxedWalletAdapter>,
    trading_key_claim: Option<TradingKeyClaim>,
}

impl<S: Signer + fmt::Debug> fmt::Debug for TxBuilder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxBuilder")
            .field("signer", &self.signer)
            .field("chain_id", &self.chain_id)
            .field("account_number", &self.account_number)
            .field("memo", &self.memo)
            .field("timeout_timestamp", &self.timeout_timestamp)
            .field("messages", &self.messages)
            .field("signing_options", &self.signing_options)
            .finish_non_exhaustive()
    }
}

impl<S: Signer> TxBuilder<S> {
    /// Creates a new builder for a local signer (Human or Agent).
    pub fn new(signer: S) -> Self {
        Self {
            signer,
            chain_id: "morpheum-test-1".to_string(),
            account_number: None,
            memo: None,
            timeout_timestamp: None,
            messages: Vec::new(),
            signing_options: SigningOptions::new(),
            nonce_provider: None,
            address_mapper: Box::new(DefaultAddressMapper),
            wallet_adapter: None,
            trading_key_claim: None,
        }
    }

    // ==================== CHAIN & ACCOUNT ====================

    /// Sets the chain ID for the transaction.
    #[must_use]
    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    /// Sets the account number for the signer.
    #[must_use]
    pub const fn account_number(mut self, account_number: u64) -> Self {
        self.account_number = Some(account_number);
        self
    }

    // ==================== GENERIC MESSAGE ADDING ====================

    /// Adds a pre-packed `prost_types::Any` message to the transaction body.
    /// This is the **only** way to add messages — keeps the signing crate 100% generic.
    #[must_use]
    pub fn add_message(mut self, msg: prost_types::Any) -> Self {
        self.messages.push(msg);
        self
    }

    /// Convenience: Adds a typed protobuf message by packing it into `Any`.
    /// The caller provides the exact type URL (e.g. "type.googleapis.com/market.v1.MsgCreateMarketRequest").
    #[must_use]
    pub fn add_typed_message<M: prost::Message>(mut self, type_url: impl Into<String>, msg: &M) -> Self {
        self.messages.push(prost_types::Any {
            type_url: type_url.into(),
            value: msg.encode_to_vec(),
        });
        self
    }

    // ==================== OPTIONS ====================

    /// Sets an optional memo on the transaction.
    #[must_use]
    pub fn memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    /// Sets a timeout (seconds since epoch) after which the transaction is invalid.
    #[must_use]
    pub const fn timeout_seconds(mut self, seconds: u64) -> Self {
        self.timeout_timestamp = Some(seconds);
        self
    }

    /// Sets signing options (deadline, memo, timestamp inclusion).
    #[must_use]
    pub fn with_signing_options(mut self, opts: SigningOptions) -> Self {
        self.signing_options = opts;
        self
    }

    // ==================== STRATEGIES ====================

    /// Injects a nonce provider strategy (Sentry, AgentPortal, etc.).
    #[must_use]
    pub fn with_nonce_provider(mut self, provider: impl NonceProvider + 'static) -> Self {
        self.nonce_provider = Some(Box::new(provider));
        self
    }

    /// Injects an external wallet adapter (MetaMask, Phantom, Taproot, etc.).
    #[must_use]
    pub fn with_wallet_adapter(mut self, adapter: impl WalletAdapter + 'static) -> Self {
        self.wallet_adapter = Some(Box::new(adapter));
        self
    }

    // ==================== AGENT-SPECIFIC ====================

    /// Attaches a `TradingKeyClaim` for agent delegation.
    #[must_use]
    pub fn with_trading_key_claim(mut self, claim: TradingKeyClaim) -> Self {
        self.trading_key_claim = Some(claim);
        self
    }

    // ==================== FINAL SIGN ====================

    /// Builds and signs the transaction.
    ///
    /// This is the only method that performs the actual signing and nonce fetching.
    pub async fn sign(self) -> Result<SignedTx, SigningError> {
        // 1. Resolve nonce
        let nonce = if let Some(provider) = &self.nonce_provider {
            provider.next_nonce(&self.signer.account_id()).await?
        } else {
            // Default fallback (for tests or offline signing)
            Nonce {
                monotonic: 0,
                ts_ms: 0,
                sub: 0,
            }
        };

        // 2. Build TxBody (messages are already Any)
        let body = TxBody {
            messages: self.messages,
            memo: self.memo.unwrap_or_default(),
            timeout_timestamp: self.timeout_timestamp.map(|ts| prost_types::Timestamp {
                seconds: ts as i64,
                nanos: 0,
            }),
        };

        // 3. Build AuthInfo + SignerInfo
        let signer_info = SignerInfo {
            public_key: Some(prost_types::Any {
                type_url: "type.googleapis.com/cosmos.crypto.ed25519.PubKey".to_string(),
                value: Vec::new(),
            }),
            mode_info: Some(ModeInfo {
                sum: Some(tx::mode_info::Sum::Single(tx::mode_info::Single {
                    mode: tx::SignMode::Ed25519 as i32,
                })),
            }),
            chain_type: 0, // filled by primitives layer if needed
            deadline: self.signing_options.deadline_seconds.unwrap_or(0),
            signing_options: None,
            timestamp: None,
        };

        let auth_info = AuthInfo {
            signer_infos: vec![signer_info],
        };

        // 4. Encode body + auth_info once (reused in SignDoc and TxRaw)
        let body_bytes = body.encode_to_vec();
        let auth_info_bytes = auth_info.encode_to_vec();

        // 5. Build SignDoc (the exact bytes that get signed)
        let sign_doc = SignDoc {
            body_bytes: body_bytes.clone(),
            auth_info_bytes: auth_info_bytes.clone(),
            chain_id: self.chain_id,
            account_number: self.account_number.unwrap_or(0),
        };

        // 6. Perform signing
        let signature = self.signer.sign(&sign_doc).await?;
        let sig_bytes = signature.to_bytes();

        // 7. Validate TradingKeyClaim if present (requires std for wall-clock time)
        #[cfg(feature = "std")]
        if let Some(ref claim) = self.trading_key_claim {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            claim.validate(now_secs)?;
        }

        // 8. Build TxRaw and Tx
        let tx_raw = TxRaw {
            body_bytes,
            auth_info_bytes,
            signatures: vec![sig_bytes.clone()],
        };

        let raw_bytes = tx_raw.encode_to_vec();

        let tx = Tx {
            body: Some(body),
            auth_info: Some(auth_info),
            signatures: vec![sig_bytes],
            nonce: Some(nonce),
            shard_id: None,
        };

        Ok(SignedTx::new(tx, raw_bytes, Some(tx_raw)))
    }
}
