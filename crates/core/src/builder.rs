//! `TxBuilder` — Fluent, generic, proto-centric transaction builder.
//!
//! This is the **main public API** of the signing library.
//! It is deliberately **completely generic** — it has no knowledge of any specific
//! module messages (MsgCreateMarketRequest, etc.). Those belong in a higher-level SDK.
//!
//! Design: Builder Pattern + Generics over `Signer` for zero-cost abstraction.

use core::fmt;

use prost::Message;

use crate::{
    canonical_sign_doc,
    claim::TradingKeyClaim,
    error::SigningError,
    mapper::{AddressMapper, DefaultAddressMapper},
    nonce::{BoxedNonceProvider, NonceProvider},
    proto::tx::v1::{self as tx, AuthInfo, ModeInfo, Nonce, SignerInfo, Tx, TxBody, TxRaw},
    signer::Signer,
    types::{SignedTx, SigningOptions},
    wallet_adapter::{BoxedWalletAdapter, WalletAdapter},
    TxGasLimit,
};

/// The gas limit a transaction declares when the caller sets none: 25,000.
///
/// Every transaction declares a gas limit in its signed `AuthInfo.gas_limit`,
/// and `0` is not a declaration (see [`TxGasLimit`]). The declaration is a cap
/// and a reservation at once: execution fails once the transaction uses more,
/// and block assembly reserves every declared unit against the block's gas
/// budget whether execution uses it or not. Declaring more than a transaction
/// needs therefore costs block space and buys nothing, so a caller that knows
/// a tighter bound should declare it with [`TxBuilder::gas_limit`].
///
/// The default fits native-module messages with a fixed cost. A message whose
/// cost grows with the work it does, and a VM message (deploying or calling a
/// contract), can need more; such a transaction must declare what it needs,
/// up to [`TX_GAS_BUDGET`](crate::TX_GAS_BUDGET), or it fails for want of gas.
pub const DEFAULT_GAS_LIMIT: TxGasLimit = match TxGasLimit::new(25_000) {
    Ok(limit) => limit,
    Err(_) => panic!("DEFAULT_GAS_LIMIT must be a valid gas-limit declaration"),
};

/// Fluent transaction builder (completely generic).
///
/// Generic over the signer to allow zero-cost monomorphization for local keys
/// while supporting dynamic dispatch for injected wallets.
pub struct TxBuilder<S: Signer> {
    signer: S,
    chain_id: String,
    /// Genesis hash of the target chain, bound into the `SignDoc` preimage so
    /// a signature cannot be replayed onto a different chain instance sharing
    /// this one's `chain_id`.
    ///
    /// Required: [`TxBuilder::sign`] refuses to build while this is empty. Set
    /// it via [`TxBuilder::with_genesis_hash`].
    genesis_hash: Vec<u8>,
    account_number: Option<u64>,
    memo: Option<String>,
    timeout_timestamp: Option<u64>,   // seconds since epoch
    messages: Vec<crate::proto::Any>, // ← ONLY generic Any
    signing_options: SigningOptions,
    nonce_provider: Option<BoxedNonceProvider>,
    manual_nonce: Option<Nonce>,
    #[allow(dead_code)]
    address_mapper: Box<dyn AddressMapper>,
    wallet_adapter: Option<BoxedWalletAdapter>,
    trading_key_claim: Option<TradingKeyClaim>,
    priority_tip: u128,
    /// Submitter-asserted semantics tier used by the chain's
    /// semantics-aware intra-block ordering.
    /// Defaults to [`morpheum_primitives::tx_class::TxClass::Standard`]
    /// (wire `0`) so call-sites that omit the field get the default
    /// ordering by construction.
    tx_class: morpheum_primitives::tx_class::TxClass,
    /// Submitter-asserted urgency hint, stamped onto `TxBody.urgent`
    /// (proto field 6). Defaults to `false`, so call-sites that omit
    /// the field submit a non-urgent transaction by construction.
    urgent: bool,
    /// The gas limit stamped onto `AuthInfo.gas_limit`, which the signature
    /// covers. [`DEFAULT_GAS_LIMIT`] until [`TxBuilder::gas_limit`] sets
    /// another; the type admits no undeclared or over-budget value.
    gas_limit: TxGasLimit,
    // Agent-specific context (optional, zero overhead for regular users).
    agent_did: Option<String>,
    verifiable_presentation: Option<Vec<u8>>,
    trading_key_address: Option<String>,
}

impl<S: Signer + fmt::Debug> fmt::Debug for TxBuilder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxBuilder")
            .field("signer", &self.signer)
            .field("chain_id", &self.chain_id)
            .field("account_number", &self.account_number)
            .field("memo", &self.memo)
            .field("timeout_timestamp", &self.timeout_timestamp)
            .field("gas_limit", &self.gas_limit)
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
            genesis_hash: Vec::new(),
            account_number: None,
            memo: None,
            timeout_timestamp: None,
            messages: Vec::new(),
            signing_options: SigningOptions::new(),
            nonce_provider: None,
            manual_nonce: None,
            address_mapper: Box::new(DefaultAddressMapper),
            wallet_adapter: None,
            trading_key_claim: None,
            priority_tip: 0,
            tx_class: morpheum_primitives::tx_class::TxClass::Standard,
            urgent: false,
            gas_limit: DEFAULT_GAS_LIMIT,
            agent_did: None,
            verifiable_presentation: None,
            trading_key_address: None,
        }
    }

    // ==================== CHAIN & ACCOUNT ====================

    /// Sets the chain ID for the transaction.
    #[must_use]
    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    /// Binds the transaction signing preimage to the target chain's genesis
    /// hash, so a signature valid on this chain cannot be replayed onto
    /// another chain sharing its `chain_id`.
    ///
    /// Required: [`sign`](Self::sign) refuses to build without it and returns
    /// [`SigningError::GenesisHashUnset`]. Take the value from operator
    /// configuration, never from the node the transaction is submitted to —
    /// whoever controls that endpoint would otherwise choose which chain the
    /// signature authorises.
    #[must_use]
    pub fn with_genesis_hash(mut self, hash: impl Into<Vec<u8>>) -> Self {
        self.genesis_hash = hash.into();
        self
    }

    /// Sets the account number for the signer.
    #[must_use]
    pub const fn account_number(mut self, account_number: u64) -> Self {
        self.account_number = Some(account_number);
        self
    }

    // ==================== GENERIC MESSAGE ADDING ====================

    /// Adds a pre-packed proto `Any` message to the transaction body.
    /// This is the **only** way to add messages — keeps the signing crate 100% generic.
    #[must_use]
    pub fn add_message(mut self, msg: crate::proto::Any) -> Self {
        self.messages.push(msg);
        self
    }

    /// Convenience: Adds a typed protobuf message by packing it into `Any`.
    /// The caller provides the exact type URL (e.g. "type.googleapis.com/market.v1.MsgCreateMarketRequest").
    #[must_use]
    pub fn add_typed_message<M: prost::Message>(
        mut self,
        type_url: impl Into<String>,
        msg: &M,
    ) -> Self {
        self.messages.push(crate::proto::Any {
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

    /// Sets an optional priority tip in oneirs (1 MORM = 10^18 oneirs) for
    /// faster inclusion during congestion. A value of 0 (default) means no
    /// tip — the transaction relies solely on mana-score sponsorship.
    /// Tips below 1 MORM are treated as dust and ignored by validators.
    #[must_use]
    pub const fn priority_tip(mut self, tip_oneirs: u128) -> Self {
        self.priority_tip = tip_oneirs;
        self
    }

    /// Declares the transaction's semantics tier for the chain's
    /// tier-aware intra-block tie-break. Leaving this unset defaults
    /// to [`morpheum_primitives::tx_class::TxClass::Standard`] (wire
    /// `0`), the default ordering.
    ///
    /// Submitter-asserted on the wire; ordering uses the declared
    /// tier but does NOT verify semantics — a mis-declared
    /// transaction is rejected at execution (a `PostOnly`
    /// that crosses, a `Cancel` against a non-existent order, etc.).
    /// See [`morpheum_primitives::tx_class`] for the encoding
    /// contract and SRP boundary.
    #[must_use]
    pub const fn with_tx_class(mut self, class: morpheum_primitives::tx_class::TxClass) -> Self {
        self.tx_class = class;
        self
    }

    /// Declares the transaction's submitter-asserted `urgent` hint.
    ///
    /// Stamped onto `TxBody.urgent` (proto field 6); signed via
    /// `SignDoc.body_bytes` so a relayer or gossip peer cannot
    /// forge it.
    ///
    /// Leaving this unset defaults to `false`, which proto3 elides
    /// from the encoding, so an unset hint adds no bytes to the
    /// transaction.
    ///
    /// The setter is `const fn` for zero-cost monomorphisation on
    /// the workload hot path; `#[must_use]` prevents the
    /// builder-misuse class where a caller forgets to bind the
    /// returned `Self`.
    #[must_use]
    pub const fn urgent(mut self, urgent: bool) -> Self {
        self.urgent = urgent;
        self
    }

    /// Declares the transaction's gas limit, replacing [`DEFAULT_GAS_LIMIT`].
    ///
    /// The limit is signed, caps the gas the transaction may use, and is
    /// reserved in full against the block's gas budget whether used or not:
    /// declare what the transaction needs, not the most it could be allowed.
    ///
    /// It takes a [`TxGasLimit`] rather than a `u64` so that an undeclared
    /// (`0`) or over-budget limit is refused by [`TxGasLimit::new`] before
    /// anything is signed, rather than by the chain after submission.
    #[must_use]
    pub const fn gas_limit(mut self, gas_limit: TxGasLimit) -> Self {
        self.gas_limit = gas_limit;
        self
    }

    /// Sets signing options (deadline, memo, timestamp inclusion).
    #[must_use]
    pub fn with_signing_options(mut self, opts: SigningOptions) -> Self {
        self.signing_options = opts;
        self
    }

    // ==================== STRATEGIES ====================

    /// Sets a pre-built nonce directly, bypassing the nonce provider.
    ///
    /// Takes precedence over any configured `NonceProvider`. Useful when the
    /// caller has already queried the nonce state (e.g. via gRPC) and wants
    /// to avoid a second round-trip.
    #[must_use]
    pub fn with_nonce(mut self, nonce: Nonce) -> Self {
        self.manual_nonce = Some(nonce);
        self
    }

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

    /// Sets the agent DID (e.g. `"did:agent:abc123…"`).
    ///
    /// Used by the chain-side auth hotpath for identity lookup and
    /// shard-affinity routing (`blake3(did)` → shard). Zero overhead
    /// when `None` (regular human transactions).
    #[must_use]
    pub fn with_agent_did(mut self, did: impl Into<String>) -> Self {
        self.agent_did = Some(did.into());
        self
    }

    /// Sets the raw Verifiable Presentation bytes.
    ///
    /// The VP is a signed bundle of claims (max daily USD, allowed pairs,
    /// etc.) verified by the VC hotpath on the chain side. Encode the
    /// `vc.v1.Vp` proto message to bytes before passing here.
    #[must_use]
    pub fn with_verifiable_presentation(mut self, vp: Vec<u8>) -> Self {
        self.verifiable_presentation = Some(vp);
        self
    }

    /// Explicitly sets the delegated trading key address.
    ///
    /// When omitted and a [`TradingKeyClaim`] is attached, the address is
    /// auto-derived from the claim's `subject` (`hex(subject.0)`).
    #[must_use]
    pub fn with_trading_key_address(mut self, addr: impl Into<String>) -> Self {
        self.trading_key_address = Some(addr.into());
        self
    }

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
    ///
    /// # Errors
    ///
    /// Returns [`SigningError::Signing`] if no messages have been added.
    pub async fn sign(self) -> Result<SignedTx, SigningError> {
        // 0. Validate: at least one message is required
        if self.messages.is_empty() {
            return Err(SigningError::signing(
                "transaction must contain at least one message",
            ));
        }

        // 0b. Refuse to build a preimage that binds no chain.
        //
        // Without a genesis hash the preimage binds no chain instance, and the
        // signature would be replayable onto any chain sharing this
        // `chain_id`. Fail closed: a signature that binds nothing is not
        // produced at all.
        //
        // ORDER IS LOAD-BEARING. This runs BEFORE the nonce is resolved. The
        // provider path (`Sentry` / `AgentPortal`) hands out a monotonic value
        // and does not take it back, so rejecting after that point would burn a
        // nonce on a transaction that never exists — leaving a gap the account's
        // monotonic sequence cannot skip past. A misconfigured caller should
        // lose nothing but the call.
        if self.genesis_hash.is_empty() {
            return Err(SigningError::GenesisHashUnset);
        }

        // 1. Resolve nonce: manual > provider > default fallback
        let nonce = if let Some(nonce) = self.manual_nonce {
            nonce
        } else if let Some(provider) = &self.nonce_provider {
            provider.next_nonce(&self.signer.account_id()).await?
        } else {
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
            timeout_timestamp: self.timeout_timestamp.map(|ts| crate::proto::Timestamp {
                seconds: ts as i64,
                nanos: 0,
            }),
            priority_tip: if self.priority_tip == 0 {
                String::new()
            } else {
                self.priority_tip.to_string()
            },
            tx_class: self.tx_class.to_wire(),
            urgent: self.urgent,
        };

        // 3. Build AuthInfo + SignerInfo
        //
        // The public key and sign mode are the signer's own, so the SignerInfo
        // always describes the key that produced the signature.

        // Auto-derive trading_key_address from TradingKeyClaim.subject when
        // the caller hasn't set it explicitly. The subject IS the trading key.
        let trading_key_address = self.trading_key_address.or_else(|| {
            self.trading_key_claim
                .as_ref()
                .map(|c| hex::encode(c.subject.0))
        });

        let mut signer_info = SignerInfo {
            public_key: Some(self.signer.public_key_proto()),
            mode_info: Some(ModeInfo {
                sum: Some(tx::mode_info::Sum::Single(tx::mode_info::Single {
                    mode: self.signer.sign_mode() as i32,
                })),
            }),
            chain_type: 0,
            deadline: self.signing_options.deadline_seconds.unwrap_or(0),
            signing_options: None,
            timestamp: None,
            agent_did: self.agent_did,
            verifiable_presentation: self.verifiable_presentation,
            trading_key_address,
        };

        // 3.5 Embed TradingKeyClaim if present.
        //
        // The claim is validated for structural correctness (expiry, nonce range,
        // signature presence) and then serialized into the `SignerInfo.signing_options`
        // field, where verifiers extract it and verify it cryptographically.
        if let Some(ref claim) = self.trading_key_claim {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            claim.validate(now_secs)?;

            let claim_any = claim.to_proto_any();
            signer_info.signing_options = Some(tx::SigningOptions {
                wasm_seed: claim_any.encode_to_vec(),
                algo_hint: "trading_key_claim".into(),
                ..Default::default()
            });
        }

        let auth_info = AuthInfo {
            signer_infos: vec![signer_info],
            gas_limit: self.gas_limit.get(),
        };

        // 4. Encode body + auth_info once (reused in SignDoc and TxRaw)
        let body_bytes = body.encode_to_vec();
        let auth_info_bytes = auth_info.encode_to_vec();

        // 5. Build SignDoc (the exact bytes that get signed) through the
        // preimage SSOT in `morpheum-primitives`. The `genesis_hash` field
        // binds the signature to a specific chain instance so a valid
        // signature cannot be replayed on another chain that happens to share
        // a `chain_id`.
        //
        // Assembled there rather than here so this signer cannot drift from
        // the verifiers: a field added to the preimage lands on both sides at
        // once, by construction.
        // The nonce resolved in step 1 is stamped onto `Tx.nonce` below, and is
        // bound here so the two cannot diverge: the signature covers the exact
        // nonce the transaction carries.
        //
        // This builder always resolves a concrete nonce (manual > provider >
        // default), so it always binds one; it never emits `None`.
        let sign_doc = canonical_sign_doc(
            body_bytes.clone(),
            auth_info_bytes.clone(),
            &self.chain_id,
            self.account_number.unwrap_or(0),
            self.genesis_hash,
            Some(nonce),
        );

        // 6. Perform signing
        let signature = self.signer.sign(&sign_doc).await?;
        let sig_bytes = signature.to_bytes();

        // 7. Build TxRaw and Tx
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
        };

        Ok(SignedTx::new(tx, raw_bytes, Some(tx_raw)))
    }
}

#[cfg(test)]
mod tests {
    //! Wire-side determinism of the signed transaction body.
    //!
    //! Asserts that `TxBuilder::priority_tip(N).sign().await` produces a
    //! signed `Tx` whose `body.priority_tip` round-trips byte-identically
    //! through prost encode → decode AND agrees with
    //! `morpheum_primitives::priority_fee::parse_tip_oneirs` for every
    //! boundary value in `{0, 1, MIN_TIP_ONEIRS, u128::MAX}`. A change that
    //! flips the `if self.priority_tip == 0 { "" } else {
    //! tip.to_string() }` branch (or a prost encoding that drops the field)
    //! would silently change the tip a signed transaction carries.

    use super::*;
    use crate::proto::tx::v1::{SignDoc, Tx as ProtoTx};
    use crate::types::AccountId;
    use crate::types::{PublicKey, Signature, WalletType};
    use async_trait::async_trait;
    use morpheum_primitives::priority_fee::{parse_tip_oneirs, MIN_TIP_ONEIRS};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// Hermetic test signer — emits a deterministic stub Ed25519 signature
    /// without invoking any crypto backend. These tests target the
    /// wire body field only; the signature path is irrelevant to the
    /// `body.priority_tip` round-trip assertion. Defined locally so the
    /// `core` crate's `#[cfg(test)]` module stays self-contained (no
    /// dev-dep on `morpheum-signing-native`, which would create a
    /// workspace-cycle in the core layer).
    struct StubSigner;

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl Signer for StubSigner {
        async fn sign(&self, _sign_doc: &SignDoc) -> Result<Signature, SigningError> {
            Ok(Signature::Ed25519([0u8; 64]))
        }

        fn public_key(&self) -> PublicKey {
            PublicKey::Ed25519([0u8; 32])
        }

        fn wallet_type(&self) -> WalletType {
            WalletType::Native
        }
    }

    fn stub_message() -> crate::proto::Any {
        crate::proto::Any {
            type_url: "type.googleapis.com/morpheum.test.v1.MsgPin".to_string(),
            value: vec![0xAA, 0xBB, 0xCC],
        }
    }

    /// Stand-in genesis hash for tests that are not about the genesis binding.
    /// `sign` refuses an unbound preimage, so every signing test must supply
    /// one; the value is irrelevant to these assertions, only its presence.
    const TEST_GENESIS_HASH: [u8; 32] = [0x5A; 32];

    /// A nonce provider that records whether it was ever consulted.
    ///
    /// Exists to prove the ordering in `sign()` step 0b, which no assertion on
    /// the returned error could establish: both orderings return the same
    /// `GenesisHashUnset`, and only the side effect distinguishes them.
    struct CountingNonceProvider {
        calls: Arc<AtomicUsize>,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl crate::nonce::NonceProvider for CountingNonceProvider {
        async fn next_nonce(&self, _account_id: &AccountId) -> Result<Nonce, SigningError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Nonce {
                monotonic: 1,
                ts_ms: 0,
                sub: 0,
            })
        }
    }

    /// A preimage that binds no chain is not produced at all.
    ///
    /// An unset genesis hash would not make signing fail on its own — it
    /// would yield a signature that binds no chain instance and is
    /// replayable onto any chain sharing this `chain_id`, with nothing about
    /// the returned `SignedTx` looking wrong. `sign` refuses instead.
    #[tokio::test]
    async fn sign_refuses_to_build_a_preimage_that_binds_no_chain() {
        let err = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .add_message(stub_message())
            .sign()
            .await
            .expect_err("an unbound preimage must not be signable");

        assert!(
            matches!(err, SigningError::GenesisHashUnset),
            "expected GenesisHashUnset, got {err:?}"
        );
    }

    /// The refusal happens BEFORE the nonce provider is consulted.
    ///
    /// Provider-backed nonces (`Sentry` / `AgentPortal`) are monotonic and are
    /// not handed back. Rejecting after fetching one would burn it on a
    /// transaction that never exists, leaving a gap the account's sequence
    /// cannot skip — so a misconfigured caller would be penalised for the
    /// misconfiguration on top of being told about it.
    #[tokio::test]
    async fn the_refusal_does_not_burn_a_monotonic_nonce() {
        let calls = Arc::new(AtomicUsize::new(0));
        let err = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .with_nonce_provider(CountingNonceProvider {
                calls: Arc::clone(&calls),
            })
            .add_message(stub_message())
            .sign()
            .await
            .expect_err("an unbound preimage must not be signable");

        assert!(matches!(err, SigningError::GenesisHashUnset));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "the nonce provider must not be consulted for a build that cannot succeed",
        );
    }

    /// A signer that records the `SignDoc` it was handed — the exact bytes its
    /// signature covers.
    struct RecordingSigner {
        signed: Arc<Mutex<Option<SignDoc>>>,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl Signer for RecordingSigner {
        async fn sign(&self, sign_doc: &SignDoc) -> Result<Signature, SigningError> {
            *self.signed.lock().expect("recorder lock") = Some(sign_doc.clone());
            Ok(Signature::Ed25519([0u8; 64]))
        }

        fn public_key(&self) -> PublicKey {
            PublicKey::Ed25519([0u8; 32])
        }

        fn wallet_type(&self) -> WalletType {
            WalletType::Native
        }
    }

    /// Every transaction declares a gas limit: one built without a declaration
    /// carries [`DEFAULT_GAS_LIMIT`], and the chain's own gas-validity
    /// predicate accepts it.
    #[tokio::test]
    async fn sign_declares_the_default_gas_limit() {
        let signed = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .with_genesis_hash(TEST_GENESIS_HASH)
            .add_message(stub_message())
            .sign()
            .await
            .expect("StubSigner build+sign should succeed");

        assert_eq!(
            TxGasLimit::declared_by(signed.tx()),
            Ok(DEFAULT_GAS_LIMIT),
            "a transaction built without a gas limit must declare the default",
        );
    }

    /// A declared gas limit is inside the bytes the signer signs, and those
    /// are the bytes the transaction ships.
    ///
    /// The declaration is a signed field: a relayer that could raise or lower
    /// it without invalidating the signature could change what the
    /// transaction is allowed to consume and how much block space it
    /// reserves. The value used is not the default, so the assertion cannot
    /// pass by the builder ignoring the setter.
    #[tokio::test]
    async fn the_declared_gas_limit_is_covered_by_the_signature() {
        let declared = TxGasLimit::MAX;
        assert_ne!(declared, DEFAULT_GAS_LIMIT);

        let recorded = Arc::new(Mutex::new(None));
        let signed = TxBuilder::new(RecordingSigner {
            signed: Arc::clone(&recorded),
        })
        .chain_id("morpheum-test-1")
        .with_genesis_hash(TEST_GENESIS_HASH)
        .add_message(stub_message())
        .gas_limit(declared)
        .sign()
        .await
        .expect("RecordingSigner build+sign should succeed");

        let sign_doc = recorded
            .lock()
            .expect("recorder lock")
            .take()
            .expect("sign() must hand the signer a SignDoc");
        let signed_auth_info = AuthInfo::decode(sign_doc.auth_info_bytes.as_slice())
            .expect("the signed auth_info_bytes must decode");
        assert_eq!(
            signed_auth_info.gas_limit,
            declared.get(),
            "the signed preimage must carry the declared gas limit",
        );

        let tx_raw = signed.tx_raw().expect("sign() must produce a TxRaw");
        assert_eq!(
            tx_raw.auth_info_bytes, sign_doc.auth_info_bytes,
            "the shipped auth_info_bytes must be the signed ones",
        );
        assert_eq!(TxGasLimit::declared_by(signed.tx()), Ok(declared));
    }

    /// Proto round-trip determinism for the four-value boundary
    /// table `tip_oneirs ∈ {0, 1, MIN_TIP_ONEIRS, u128::MAX}`.
    ///
    /// Steps per row:
    /// 1. Build + sign with the stub signer at the given tip.
    /// 2. Assert `signed.tx().body.priority_tip` matches the wire-omission
    ///    convention at `builder.rs:317-321` (`""` for `0`, `N.to_string()`
    ///    otherwise).
    /// 3. Round-trip via prost: `signed.tx().encode_to_vec()` →
    ///    `ProtoTx::decode` → assert the decoded `body.priority_tip` is
    ///    byte-identical to the pre-encode value.
    /// 4. Assert `parse_tip_oneirs(&decoded.body.priority_tip).unwrap_or(0)`
    ///    equals `N` (the shared primitives parser agrees with this
    ///    encoder for every boundary value).
    #[tokio::test]
    async fn priority_tip_round_trips_through_prost_for_boundary_values() {
        const TABLE: [u128; 4] = [0u128, 1u128, MIN_TIP_ONEIRS, u128::MAX];

        for &tip_oneirs in &TABLE {
            let signed = TxBuilder::new(StubSigner)
                .chain_id("morpheum-test-1")
                .with_genesis_hash(TEST_GENESIS_HASH)
                .add_message(stub_message())
                .priority_tip(tip_oneirs)
                .sign()
                .await
                .expect("StubSigner build+sign should succeed");

            let body = signed
                .tx()
                .body
                .as_ref()
                .expect("signed Tx must carry a body");

            let expected_wire = if tip_oneirs == 0 {
                String::new()
            } else {
                tip_oneirs.to_string()
            };
            assert_eq!(
                body.priority_tip, expected_wire,
                "in-memory Tx body priority_tip must match the wire-omission convention for tip_oneirs={tip_oneirs}",
            );

            let encoded = signed.tx().encode_to_vec();
            let decoded =
                ProtoTx::decode(encoded.as_slice()).expect("Tx must decode after prost round-trip");
            let decoded_body = decoded.body.as_ref().expect("decoded Tx must carry a body");

            assert_eq!(
                decoded_body.priority_tip, expected_wire,
                "decoded body.priority_tip must be byte-identical to the encoded value for tip_oneirs={tip_oneirs}",
            );

            let parsed = parse_tip_oneirs(&decoded_body.priority_tip)
                .expect("parse_tip_oneirs must succeed on every encoder output");
            assert_eq!(
                parsed, tip_oneirs,
                "parse_tip_oneirs must agree with the encoder for tip_oneirs={tip_oneirs}",
            );
        }
    }

    /// Canonical proto3 wire-byte triple for `TxBody.priority_tip = "1"`
    /// — `[tag=0x22, len=0x01, ascii_one=0x31]` (field 4, length-delimited,
    /// the one-byte string `"1"`). Written out literally so the expectation
    /// is stated independently of the encoder under test.
    const PRIORITY_TIP_ONE_WIRE_TRIPLE: [u8; 3] = [0x22, 0x01, 0x31];

    /// Full-encoder pin asserting that
    /// `TxBuilder::priority_tip(1).sign()` produces a `Tx` whose
    /// **fully-encoded prost wire bytes** (the exact bytes that go
    /// out over the gRPC `submit_tx` channel) contain the canonical
    /// triple `[0x22, 0x01, 0x31]` exactly once.
    ///
    /// **Why this goes beyond the round-trip test above.** That test
    /// asserts the round-trip on `signed.tx().body.priority_tip`
    /// (string field). This one closes the next layer: even if
    /// `body.priority_tip` is `"1"` in memory, a change that mis-tags
    /// the field on the wire (e.g. a stale `morpheum-proto` $OUT_DIR
    /// cache linked against the signing crate, a custom `Encode` impl
    /// that drops the field, a hypothetical `#[prost(skip)]`
    /// annotation) would pass the round-trip test but fail this one.
    #[tokio::test]
    async fn priority_tip_one_encodes_the_canonical_wire_triple_once() {
        let signed = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .with_genesis_hash(TEST_GENESIS_HASH)
            .add_message(stub_message())
            .priority_tip(1)
            .sign()
            .await
            .expect("StubSigner build+sign should succeed for tip_oneirs=1");

        let encoded = signed.tx().encode_to_vec();
        let occurrences = encoded
            .windows(PRIORITY_TIP_ONE_WIRE_TRIPLE.len())
            .filter(|w| *w == PRIORITY_TIP_ONE_WIRE_TRIPLE)
            .count();

        assert_eq!(
            occurrences, 1,
            "TxBuilder::priority_tip(1).sign().tx().encode_to_vec() must contain the \
             canonical wire triple [0x22, 0x01, 0x31] exactly once (proto3 field 4, \
             length-delimited string \"1\"). Got {occurrences} occurrences in encoded bytes \
             {encoded:?}.",
        );
    }

    /// Negative symmetry — `TxBuilder::priority_tip(0).sign()`
    /// MUST produce a wire stream that does **NOT** contain the
    /// canonical tipped triple. Locks the wire-omission convention
    /// (proto3 default-value elision) on the full encoder:
    /// "untipped tx" ⇔ "no `[0x22, 0x01, 0x31]` on the wire".
    ///
    /// Without this negative pin, a hypothetical change that
    /// always emits `priority_tip = "1"` (regardless of caller
    /// intent) would silently tip every transaction the caller
    /// meant to leave untipped.
    #[tokio::test]
    async fn priority_tip_zero_omits_the_canonical_wire_triple() {
        let signed = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .with_genesis_hash(TEST_GENESIS_HASH)
            .add_message(stub_message())
            .priority_tip(0)
            .sign()
            .await
            .expect("StubSigner build+sign should succeed for tip_oneirs=0");

        let encoded = signed.tx().encode_to_vec();
        let occurrences = encoded
            .windows(PRIORITY_TIP_ONE_WIRE_TRIPLE.len())
            .filter(|w| *w == PRIORITY_TIP_ONE_WIRE_TRIPLE)
            .count();

        assert_eq!(
            occurrences, 0,
            "TxBuilder::priority_tip(0).sign().tx().encode_to_vec() must not contain the \
             canonical wire triple [0x22, 0x01, 0x31] anywhere (proto3 elides default-value \
             strings, so an untipped transaction carries no tip on the wire). Got \
             {occurrences} occurrences in encoded bytes {encoded:?}.",
        );
    }

    /// **`TxBuilder::urgent` round-trip.**
    ///
    /// `TxBuilder::urgent(true).sign()` MUST produce a signed
    /// `Tx` whose `body.urgent == true` both in memory and after
    /// prost encode → decode (catches a change where the field is
    /// stripped on the wire), and `urgent(false)` MUST round-trip
    /// as `false`.
    ///
    /// Without this pin, the `TxBuilder::urgent` setter could
    /// become a no-op (e.g. a refactor that drops the
    /// `urgent: self.urgent` wire-up from the `TxBody { .. }`
    /// literal) and every transaction would still ship
    /// `body.urgent = false` regardless of what the caller asked for.
    #[tokio::test]
    async fn urgent_flag_round_trips_on_the_wire() {
        let signed_urgent = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .with_genesis_hash(TEST_GENESIS_HASH)
            .add_message(stub_message())
            .urgent(true)
            .sign()
            .await
            .expect("StubSigner build+sign should succeed for urgent=true");
        let body_urgent = signed_urgent
            .tx()
            .body
            .as_ref()
            .expect("signed urgent Tx must carry a body");
        assert!(
            body_urgent.urgent,
            "TxBuilder::urgent(true).sign().tx().body.urgent must be true: the builder \
             must carry the caller's urgent flag into the TxBody it signs."
        );
        let encoded_urgent = signed_urgent.tx().encode_to_vec();
        let decoded_urgent = ProtoTx::decode(encoded_urgent.as_slice())
            .expect("encoded urgent Tx must decode after prost round-trip");
        assert!(
            decoded_urgent
                .body
                .as_ref()
                .expect("decoded urgent Tx must carry a body")
                .urgent,
            "decoded body.urgent must be true after prost encode → decode: \
             an urgent flag set to true must survive the wire encoding."
        );

        // Negative-symmetry: urgent=false MUST elide on the wire
        // (proto3 default-elision) and round-trip cleanly via the
        // proto3 default-value path.
        let signed_non_urgent = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .with_genesis_hash(TEST_GENESIS_HASH)
            .add_message(stub_message())
            .urgent(false)
            .sign()
            .await
            .expect("StubSigner build+sign should succeed for urgent=false");
        let body_non_urgent = signed_non_urgent
            .tx()
            .body
            .as_ref()
            .expect("signed non-urgent Tx must carry a body");
        assert!(
            !body_non_urgent.urgent,
            "TxBuilder::urgent(false).sign().tx().body.urgent must be false."
        );
        let encoded_non_urgent = signed_non_urgent.tx().encode_to_vec();
        let decoded_non_urgent = ProtoTx::decode(encoded_non_urgent.as_slice())
            .expect("encoded non-urgent Tx must decode after prost round-trip");
        assert!(
            !decoded_non_urgent
                .body
                .as_ref()
                .expect("decoded non-urgent Tx must carry a body")
                .urgent,
            "decoded body.urgent must be false after prost encode → decode \
             (proto3 elides the default value, which decodes as false)."
        );
    }
}
