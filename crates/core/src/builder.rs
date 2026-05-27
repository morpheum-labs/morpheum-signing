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
    /// Optional genesis hash (raw bytes, typically a SHA-256 digest of the
    /// target chain's genesis block) that will be bound into the `SignDoc`
    /// preimage. Phase M3 (`O20` / audit row `C12`): defaults to an empty
    /// byte string for backward compatibility with pre-fork signers; callers
    /// targeting the strict-binding fork MUST set it explicitly via
    /// [`TxBuilder::with_genesis_hash`].
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
    /// Submitter-asserted semantics tier consumed by the
    /// consensus tie-break (Phase 23A — semantics-aware ordering).
    /// Defaults to [`morpheum_primitives::tx_class::TxClass::Standard`]
    /// (wire `0`) so pre-23A call-sites that omit the field inherit
    /// the legacy ordering by construction.
    tx_class: morpheum_primitives::tx_class::TxClass,
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
    /// hash (Phase M3 — audit `O20` / row `C12`).
    ///
    /// Leaving this unset produces a `SignDoc` with an empty `genesis_hash`
    /// byte string, which remains valid pre-fork. At/after the strict-binding
    /// fork activates, callers MUST set the correct genesis hash or the
    /// resulting signature will be rejected by `verify_tx`.
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

    /// Declares the transaction's semantics tier for the Phase 23A
    /// tier-aware intra-block tie-break. Leaving this unset defaults
    /// to [`morpheum_primitives::tx_class::TxClass::Standard`] (wire
    /// `0`), which matches pre-23A behavior.
    ///
    /// Submitter-asserted on the wire; the consensus crate orders by
    /// tier but does NOT verify semantics — the runtime executor
    /// rejects mis-declared transactions at execution (a `PostOnly`
    /// that crosses, a `Cancel` against a non-existent order, etc.).
    /// See [`morpheum_primitives::tx_class`] for the encoding
    /// contract and SRP boundary.
    #[must_use]
    pub const fn with_tx_class(
        mut self,
        class: morpheum_primitives::tx_class::TxClass,
    ) -> Self {
        self.tx_class = class;
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
        };

        // 3. Build AuthInfo + SignerInfo
        //
        // With `dynamic-signer-info`: public key and sign mode are derived from
        // the signer's actual key type (fixes Critical Issue #1 from audit).
        // Without: falls back to legacy hardcoded ed25519 for backward compat.

        // Auto-derive trading_key_address from TradingKeyClaim.subject when
        // the caller hasn't set it explicitly. The subject IS the trading key.
        let trading_key_address = self.trading_key_address.or_else(|| {
            self.trading_key_claim
                .as_ref()
                .map(|c| hex::encode(c.subject.0))
        });

        #[cfg(feature = "dynamic-signer-info")]
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

        #[cfg(not(feature = "dynamic-signer-info"))]
        let mut signer_info = SignerInfo {
            public_key: Some(crate::proto::Any {
                type_url: "/morpheum.crypto.ed25519.PubKey".to_string(),
                value: Vec::new(),
            }),
            mode_info: Some(ModeInfo {
                sum: Some(tx::mode_info::Sum::Single(tx::mode_info::Single {
                    mode: tx::SignMode::Ed25519 as i32,
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

        // 3.5 Embed TradingKeyClaim if present (fixes Critical Issue #2).
        //
        // The claim is validated for structural correctness (expiry, nonce range,
        // signature presence) and then serialized into the `SignerInfo.signing_options`
        // field. The chain-side extracts and cryptographically verifies the claim
        // via the VC hot-path.
        if let Some(ref claim) = self.trading_key_claim {
            #[cfg(feature = "std")]
            {
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                claim.validate(now_secs)?;
            }

            let claim_any = claim.to_proto_any();
            signer_info.signing_options = Some(tx::SigningOptions {
                wasm_seed: claim_any.encode_to_vec(),
                algo_hint: "trading_key_claim".into(),
                ..Default::default()
            });
        }

        let auth_info = AuthInfo {
            signer_infos: vec![signer_info],
            gas_limit: 0,
        };

        // 4. Encode body + auth_info once (reused in SignDoc and TxRaw)
        let body_bytes = body.encode_to_vec();
        let auth_info_bytes = auth_info.encode_to_vec();

        // 5. Build SignDoc (the exact bytes that get signed). The
        // `genesis_hash` field (Phase M3 — `O20` / `C12`) binds the signature
        // to a specific chain instance so a valid signature cannot be
        // replayed on a forked chain that happens to share a `chain_id`.
        let sign_doc = SignDoc {
            body_bytes: body_bytes.clone(),
            auth_info_bytes: auth_info_bytes.clone(),
            chain_id: self.chain_id,
            account_number: self.account_number.unwrap_or(0),
            genesis_hash: self.genesis_hash,
        };

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
    //! Phase 22X.4.5 Pin A — bench wire-side determinism.
    //!
    //! Asserts that `TxBuilder::priority_tip(N).sign().await` produces a
    //! signed `Tx` whose `body.priority_tip` round-trips byte-identically
    //! through prost encode → decode AND agrees with
    //! `morpheum_primitives::priority_fee::parse_tip_oneirs` for every
    //! boundary value in `{0, 1, MIN_TIP_ONEIRS, u128::MAX}`. A regression
    //! that flips the `if self.priority_tip == 0 { "" } else {
    //! tip.to_string() }` branch at line 317-321 (or that the prost wire
    //! drops the field) silently zeros downstream emission gates and
    //! breaks every `consensus.economics.*` MEV gauge — see
    //! [`mormcore/docs/consensus/bench/phase22x4-5-stage-0-scope.md`](../../../mormcore/docs/consensus/bench/phase22x4-5-stage-0-scope.md)
    //! §3 for the H2 hypothesis taxonomy.

    use super::*;
    use crate::proto::tx::v1::Tx as ProtoTx;
    use crate::types::{PublicKey, Signature, WalletType};
    use async_trait::async_trait;
    use morpheum_primitives::priority_fee::{parse_tip_oneirs, MIN_TIP_ONEIRS};

    /// Hermetic test signer — emits a deterministic stub Ed25519 signature
    /// without invoking any crypto backend. The Pin A contract targets the
    /// wire body field only; the signature path is irrelevant to the
    /// `body.priority_tip` round-trip assertion. Defined locally so the
    /// `core` crate's `#[cfg(test)]` module stays self-contained (no
    /// dev-dep on `morpheum-signing-native`, which would create a
    /// workspace-cycle in the no_std core layer).
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

    /// Pin A — proto round-trip determinism for the four-value boundary
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
    ///    equals `N` (the chain-side admission helper agrees with the
    ///    bench-side encoder for every boundary value).
    #[tokio::test]
    async fn phase22x4_5_pin_a_priority_tip_round_trips_through_prost_for_boundary_table() {
        const TABLE: [u128; 4] = [0u128, 1u128, MIN_TIP_ONEIRS, u128::MAX];

        for &tip_oneirs in &TABLE {
            let signed = TxBuilder::new(StubSigner)
                .chain_id("morpheum-test-1")
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
            let decoded = ProtoTx::decode(encoded.as_slice())
                .expect("Tx must decode after prost round-trip");
            let decoded_body = decoded
                .body
                .as_ref()
                .expect("decoded Tx must carry a body");

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
    /// — `[tag=0x22, len=0x01, ascii_one=0x31]`. SSOT-mirrored from
    /// [`mormcore::consensus::metrics_self_diagnostic::PRIORITY_TIP_ONE_TAG_4_WIRE`]
    /// (the chain-side admission scanner reuses the same triple).
    /// Inlined here because `morpheum-signing-core` is upstream of
    /// `morpheum-consensus` in the workspace dependency graph; a
    /// proto-level edit that breaks the derivation flips the build-
    /// time invariant in
    /// [`morpheum-proto/tests/priority_tip_wire_tag_invariant.rs`](../../../../morpheum-proto/tests/priority_tip_wire_tag_invariant.rs)
    /// before this constant can ever drift silently.
    const PIN_L_PRIORITY_TIP_ONE_TAG_4_WIRE: [u8; 3] = [0x22, 0x01, 0x31];

    /// Pin L — full bench-side encoder pin asserting that
    /// `TxBuilder::priority_tip(1).sign()` produces a `Tx` whose
    /// **fully-encoded prost wire bytes** (the exact bytes that go
    /// out over the gRPC `submit_tx` channel) contain the canonical
    /// triple `[0x22, 0x01, 0x31]` exactly once.
    ///
    /// **Why this strictly subsumes Pin A.** Pin A asserts the
    /// round-trip on `signed.tx().body.priority_tip` (string field).
    /// Pin L closes the next layer: even if `body.priority_tip` is
    /// `"1"` in memory, a regression that mis-tags the field on the
    /// wire (e.g. a stale `morpheum-proto` $OUT_DIR cache linked
    /// against the signing crate, a custom `Encode` impl that drops
    /// the field, a hypothetical `#[prost(skip)]` annotation) would
    /// pass Pin A but fail Pin L. Pin L is the bench-side mirror of
    /// the chain-side admission scanner's invariant — the two
    /// bridge the bench → wire → chain pipeline at byte-identity.
    ///
    /// **§2.13 forensic context.** The Phase 22X.4.7 §2.13 Pin J
    /// drive observed
    /// `consensus.ingress.admission_payload_priority_tip_one_marker_present_count`
    /// Σ=0 across every validator pod despite the bench configuring
    /// `MORM_BENCH_MEV_EXTRACTION_TIP_INTERLEAVE_N=1` +
    /// `_TIP_ONEIRS=1`. Pin L is the unit-local pre-flight pin
    /// that catches the regression class **before** the operator
    /// pays the cluster bring-up cost.
    #[tokio::test]
    async fn phase22x4_7_stage_3_e_x_pin_l_priority_tip_one_emits_canonical_wire_triple() {
        let signed = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .add_message(stub_message())
            .priority_tip(1)
            .sign()
            .await
            .expect("StubSigner build+sign should succeed for tip_oneirs=1");

        let encoded = signed.tx().encode_to_vec();
        let occurrences = encoded
            .windows(PIN_L_PRIORITY_TIP_ONE_TAG_4_WIRE.len())
            .filter(|w| *w == PIN_L_PRIORITY_TIP_ONE_TAG_4_WIRE)
            .count();

        assert_eq!(
            occurrences, 1,
            "Pin L: TxBuilder::priority_tip(1).sign().tx().encode_to_vec() MUST contain the \
             canonical wire triple [0x22, 0x01, 0x31] exactly once (proto3 tag-4 LEN-delimited \
             string \"1\"). Got {occurrences} occurrences in encoded bytes {encoded:?}. \
             Remediation tree: \
             (1) `morpheum-proto/tests/priority_tip_wire_tag_invariant.rs::txbody_priority_tip_one_encodes_canonical_tag_4_wire_triple` \
             also red → proto edit reassigned field-number 4; revert or re-tag. \
             (2) Proto pin GREEN but Pin L red → bench-side `TxBuilder::sign()` is dropping or \
             reshaping `body.priority_tip` between the in-memory body and the encoded `Tx` \
             (audit `builder.rs:317-321` for an off-by-one branch flip on the \
             `if self.priority_tip == 0` guard).",
        );
    }

    /// Pin L (negative-symmetry) — `TxBuilder::priority_tip(0).sign()`
    /// MUST produce a wire stream that does **NOT** contain the
    /// canonical tipped triple. Locks the wire-omission convention
    /// (proto3 default-value elision) on the full bench encoder so
    /// the marker scan stays bit-identity with admission semantics:
    /// "untipped tx" ⇔ "no `[0x22, 0x01, 0x31]` on the wire".
    ///
    /// Without this negative pin, a hypothetical regression that
    /// always emits `priority_tip = "1"` (regardless of caller
    /// intent) would silently flip every untipped admission into
    /// the tipped wire-byte sentinel's positive distribution and
    /// alias the §5.2O matrix's tipped-vs-untipped strata.
    #[tokio::test]
    async fn phase22x4_7_stage_3_e_x_pin_l_priority_tip_zero_omits_canonical_wire_triple() {
        let signed = TxBuilder::new(StubSigner)
            .chain_id("morpheum-test-1")
            .add_message(stub_message())
            .priority_tip(0)
            .sign()
            .await
            .expect("StubSigner build+sign should succeed for tip_oneirs=0");

        let encoded = signed.tx().encode_to_vec();
        let occurrences = encoded
            .windows(PIN_L_PRIORITY_TIP_ONE_TAG_4_WIRE.len())
            .filter(|w| *w == PIN_L_PRIORITY_TIP_ONE_TAG_4_WIRE)
            .count();

        assert_eq!(
            occurrences, 0,
            "Pin L (negative): TxBuilder::priority_tip(0).sign().tx().encode_to_vec() MUST NOT \
             contain the canonical wire triple [0x22, 0x01, 0x31] anywhere (proto3 elides \
             default-value strings). Got {occurrences} occurrences in encoded bytes {encoded:?}. \
             A non-zero count here means the encoder is shipping `priority_tip = \"1\"` for \
             tip_oneirs=0, which would alias every untipped admission into the wire-byte \
             sentinel's tipped distribution and structurally break the §5.2O Pin J matrix's \
             tipped-vs-untipped strata.",
        );
    }
}
