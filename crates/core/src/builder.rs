//! TxBuilder — Fluent, generic, proto-centric transaction builder.
//!
//! This is the main public API of the signing library.
//! It unifies signing for humans (sequential nonce) and agents (TradingKey + VC claims)
//! while supporting injected wallets (MetaMask, Phantom, Taproot, etc.).
//!
//! Design: Builder Pattern + Generics over `Signer` for zero-cost abstraction.

use alloc::vec::Vec;
use core::fmt;

use crate::{
    claim::TradingKeyClaim,
    error::SigningError,
    mapper::{AddressMapper, DefaultAddressMapper},
    nonce::{BoxedNonceProvider, NonceProvider},
    proto::{AuthInfo, ModeInfo, Nonce, SignDoc, SignerInfo, Tx, TxBody, TxRaw},
    signer::Signer,
    types::{AccountId, Address, PublicKey, Signature, SignedTx, SigningOptions, WalletType},
    wallet_adapter::BoxedWalletAdapter,
};

/// Fluent transaction builder.
///
/// Generic over the signer to allow zero-cost monomorphization for local keys
/// while supporting dynamic dispatch for injected wallets.
#[derive(Debug)]
pub struct TxBuilder<S: Signer> {
    signer: S,
    chain_id: String,
    account_number: Option<u64>,
    memo: Option<String>,
    timeout_timestamp: Option<u64>, // seconds since epoch
    messages: Vec<prost_types::Any>,
    signing_options: SigningOptions,
    // Strategies (injected via .with_* methods)
    nonce_provider: Option<BoxedNonceProvider>,
    address_mapper: Box<dyn AddressMapper>,
    wallet_adapter: Option<BoxedWalletAdapter>,
    trading_key_claim: Option<TradingKeyClaim>,
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

    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    pub fn account_number(mut self, account_number: u64) -> Self {
        self.account_number = Some(account_number);
        self
    }

    // ==================== COMMON MESSAGE HELPERS ====================

    pub fn create_market(mut self, req: crate::proto::market::v1::MsgCreateMarketRequest) -> Self {
        self.messages.push(req.into_any());
        self
    }

    pub fn place_order(mut self, req: crate::proto::clob::v1::MsgPlaceOrder) -> Self {
        self.messages.push(req.into_any());
        self
    }

    // Generic fallback for any message
    pub fn add_message(mut self, msg: prost_types::Any) -> Self {
        self.messages.push(msg);
        self
    }

    // ==================== OPTIONS ====================

    pub fn memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn timeout_seconds(mut self, seconds: u64) -> Self {
        self.timeout_timestamp = Some(seconds);
        self
    }

    pub fn with_signing_options(mut self, opts: SigningOptions) -> Self {
        self.signing_options = opts;
        self
    }

    // ==================== NONCE STRATEGY ====================

    pub fn with_nonce_provider(mut self, provider: impl NonceProvider + 'static) -> Self {
        self.nonce_provider = Some(Box::new(provider));
        self
    }

    // ==================== WALLET ADAPTER (for injected wallets) ====================

    pub fn with_wallet_adapter(mut self, adapter: impl WalletAdapter + 'static) -> Self {
        self.wallet_adapter = Some(Box::new(adapter));
        self
    }

    // ==================== AGENT-SPECIFIC ====================

    pub fn with_trading_key_claim(mut self, claim: TradingKeyClaim) -> Self {
        self.trading_key_claim = Some(claim);
        self
    }

    // ==================== FINAL SIGN ====================

    /// Builds and signs the transaction.
    ///
    /// This is the only method that performs the actual signing and nonce fetching.
    pub async fn sign(mut self) -> Result<SignedTx, SigningError> {
        // 1. Resolve nonce
        let nonce = if let Some(provider) = &self.nonce_provider {
            provider.next_nonce(&self.signer.account_id()).await?
        } else {
            // Default fallback (for tests or offline)
            Nonce {
                monotonic: 0,
                ts_ms: 0,
                sub: 0,
            }
        };

        // 2. Build TxBody
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
                type_url: "type.googleapis.com/cosmos.crypto.ed25519.PubKey".to_string(), // example
                value: vec![], // populated by primitives in full integration
            }),
            mode_info: Some(ModeInfo {
                sum: Some(proto::mode_info::Sum::Single(proto::mode_info::Single {
                    mode: proto::SignMode::SignModeDirect as i32,
                })),
            }),
            chain_type: 0, // filled by primitives
            deadline: self.signing_options.deadline_seconds.unwrap_or(0),
            signing_options: None,
            timestamp: None,
        };

        let auth_info = AuthInfo {
            signer_infos: vec![signer_info],
        };

        // 4. Build SignDoc (the exact bytes that get signed)
        let sign_doc = SignDoc {
            body_bytes: body.encode_to_vec(),
            auth_info_bytes: auth_info.encode_to_vec(),
            chain_id: self.chain_id,
            account_number: self.account_number.unwrap_or(0),
        };

        // 5. Perform signing
        let signature = self.signer.sign(&sign_doc).await?;

        // 6. Build TxRaw and Tx
        let tx_raw = TxRaw {
            body_bytes: body.encode_to_vec(),
            auth_info_bytes: auth_info.encode_to_vec(),
            signatures: vec![signature.0.clone()],
        };

        let mut tx = Tx {
            body,
            auth_info,
            signatures: vec![signature.0],
            nonce,
            shard_id: None,
        };

        // 7. Embed TradingKeyClaim if present
        if let Some(claim) = self.trading_key_claim {
            // In real code this would be embedded in AuthInfo.signer_infos
            // For now we just validate it
            claim.validate(chrono::Utc::now().timestamp() as u64)?;
        }

        // 8. Assemble final SignedTx
        let raw_bytes = tx_raw.encode_to_vec();

        Ok(SignedTx::new(tx, raw_bytes, Some(tx_raw)))
    }
}

// ==================== CONVENIENCE CONSTRUCTORS ====================

impl TxBuilder<crate::signer::HumanSigner> {
    pub fn human(signer: crate::signer::HumanSigner) -> Self {
        Self::new(signer)
    }
}

impl TxBuilder<crate::signer::AgentSigner> {
    pub fn agent(signer: crate::signer::AgentSigner) -> Self {
        Self::new(signer)
    }
}