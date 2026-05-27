//! Cross-crate integration tests: morpheum-standards ↔ morpheum-signing.
//!
//! These tests verify the complete end-to-end flow between the cryptogram
//! ecosystem (message construction, EIP-712, payload builders) and the
//! signing SDK (TxBuilder, signers, verification).
//!
//! **What this catches**:
//! - Regressions when standards payload formats change but signing still works.
//! - Full roundtrip: build protocol message → sign → verify cryptographically.
//! - Agent delegation + TradingKeyClaim → embedded and extracted correctly.
//! - Cross-curve verification (Ed25519, Secp256k1) via `verify_signed_tx`.
//!
//! Run with: `cargo test -p morpheum-signing-native --test integration cross_crate_signing --all-features`

use super::common::*;
use morpheum_signing_core::signer::Signer;
use morpheum_signing_native::prelude::*;

// Standards APIs via the morpheum-crypto meta-crate (dev-dependency).
use morpheum_crypto::standards::msgfactory::MessageFactory;
use morpheum_crypto::standards::registry::{
    create_typed_data_from_payload, register_all_actions, ActionRegistry,
};
use morpheum_crypto::types::domain_testnet;

// ==================== HELPERS ====================

/// Creates a simple bank transfer payload using the standards `Payload` format.
fn bank_transfer_payload(from: &str, to: &str, amount: &str) -> Payload {
    use serde_json::json;
    [
        ("action", json!("bank::transfer")),
        ("owner", json!(from)),
        ("toAddress", json!(to)),
        ("assetIndex", json!(0u64)),
        ("amount", json!(amount)),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.clone()))
    .collect()
}

/// Wraps a JSON-serializable payload into a proto `Any` message.
fn payload_to_any(type_url: &str, payload: &Payload) -> Any {
    Any {
        type_url: type_url.to_string(),
        value: serde_json::to_vec(payload).expect("payload serialization"),
    }
}

/// Creates an `AgentSigner` with a `TradingKeyClaim` whose issuer matches
/// the signer's public key — required for `verify_signed_tx` with `claim-verification`.
fn create_agent_with_matching_claim(seed: &[u8; 32]) -> (AgentSigner, TradingKeyClaim) {
    // Derive the public key AccountId first (this is what the verifier checks).
    let temp = AgentSigner::new(seed, AccountId::ZERO, None);
    let pubkey_account_id = temp.public_key().to_account_id();

    let now = now_secs();
    let claim = VcClaimBuilder::new()
        .issuer(pubkey_account_id.clone())
        .subject(pubkey_account_id.clone())
        .permissions(1 << 0) // TRADE
        .max_daily_usd(1_000_000)
        .expiry(now + 86_400) // 24 hours
        .nonce_sub_range(1000, 2000)
        .signature(Signature::Ed25519([1u8; 64])) // non-zero dummy for test
        .build(now)
        .expect("claim build");

    let signer = AgentSigner::new(seed, pubkey_account_id, Some(claim.clone()));
    (signer, claim)
}

// ==================== STANDARDS PAYLOAD → SIGNING SDK ====================

/// OrderBuilder (standards) → TxBuilder (signing) → SignedTx → verify_signed_tx.
///
/// This is the canonical cross-crate flow: the standards crate builds a CLOB
/// order payload, and the signing SDK turns it into a signed, verifiable transaction.
#[tokio::test]
async fn test_order_payload_to_native_signing_roundtrip() {
    // 1. Build a CLOB limit order payload via standards' OrderBuilder
    let payload = OrderBuilder::new(
        "morpheum1testowner",
        "morpheum1testaddress",
        "42",
        "9999999999",
        "1",
        OrderSide::Buy,
        OrderType::Limit,
        "1000",
    )
    .with_price("50000")
    .build()
    .expect("OrderBuilder should produce a valid payload");

    // 2. Wrap payload in proto Any
    let msg = payload_to_any(
        "type.googleapis.com/clob.v1.MsgSubmitOrderRequest",
        &payload,
    );

    // 3. Sign via signing SDK
    let signer = NativeSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 100 };

    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Cross-crate: OrderBuilder → NativeSigner")
        .with_nonce_provider(nonce_provider)
        .add_message(msg)
        .sign()
        .await
        .expect("signing with standards payload failed");

    // 4. Structure assertions
    assert!(!signed_tx.raw_bytes.is_empty());
    let body = signed_tx.tx.body.as_ref().unwrap();
    assert_eq!(body.messages.len(), 1);
    assert_eq!(
        body.messages[0].type_url,
        "type.googleapis.com/clob.v1.MsgSubmitOrderRequest"
    );
    assert_eq!(body.memo, "Cross-crate: OrderBuilder → NativeSigner");
    assert_eq!(signed_tx.tx.nonce.as_ref().unwrap().monotonic, 100);

    // 5. Verify payload integrity through the roundtrip
    let decoded: Payload =
        serde_json::from_slice(&body.messages[0].value).expect("payload deserialization");
    assert_eq!(
        decoded.get("action").and_then(|v| v.as_str()),
        Some("clob::submit_order")
    );
    assert_eq!(decoded.get("price").and_then(|v| v.as_str()), Some("50000"));
    assert_eq!(decoded.get("side").and_then(|v| v.as_str()), Some("buy"));
    assert_eq!(
        decoded.get("quantity").and_then(|v| v.as_str()),
        Some("1000")
    );

    // 6. Cryptographic roundtrip verification
    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("verify_signed_tx failed");
    assert_eq!(verified.account_ids.len(), 1);
    assert_eq!(verified.wallet_type, WalletType::Native);
    assert_eq!(verified.sign_mode, SignMode::Ed25519);
    assert!(verified.trading_key_claim.is_none());
}

/// MessageFactory (standards) → Eip712Tx → Any → TxBuilder (signing) → SignedTx → verify.
///
/// Demonstrates that the standards' EIP-712 message factory integrates seamlessly
/// with the signing SDK's transaction building and verification pipeline.
#[tokio::test]
async fn test_msgfactory_eip712_to_signing() {
    // 1. Build an EIP-712 message via MessageFactory
    let domain = domain_testnet();
    let factory = MessageFactory::new(domain);

    let mut payload: Payload = serde_json::Map::new();
    payload.insert("owner".into(), serde_json::json!("morpheum1abc"));
    payload.insert("toAddress".into(), serde_json::json!("morpheum1xyz"));
    payload.insert("amount".into(), serde_json::json!("1000000"));
    payload.insert("assetIndex".into(), serde_json::json!(0u64));

    let eip712_tx = factory
        .create_single_sign_message("bank::transfer", payload, SigType::Ed25519)
        .expect("MessageFactory should produce a valid Eip712Tx");

    // 2. Serialize the Eip712Tx and wrap as Any
    let tx_bytes = serde_json::to_vec(&eip712_tx).expect("Eip712Tx serialization");
    let msg = Any {
        type_url: "type.googleapis.com/standards.v1.Eip712Tx".to_string(),
        value: tx_bytes,
    };

    // 3. Sign via signing SDK
    let signer = NativeSigner::from_seed(&TEST_SEED);
    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Cross-crate: MessageFactory → signing")
        .add_message(msg)
        .sign()
        .await
        .expect("signing failed");

    // 4. Verify structure
    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(signed_tx.tx.body.as_ref().unwrap().messages.len(), 1);
    assert_eq!(signed_tx.tx.signatures.len(), 1);
    assert!(!signed_tx.tx.signatures[0].is_empty());

    // 5. Cryptographic verification
    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("roundtrip verification failed");
    assert_eq!(verified.account_ids.len(), 1);
}

/// Bank transfer payload → signing → verify → account identity preserved.
#[tokio::test]
async fn test_bank_transfer_payload_roundtrip() {
    let payload = bank_transfer_payload("morpheum1sender", "morpheum1receiver", "500000");
    let msg = payload_to_any("type.googleapis.com/bank.v1.MsgTransferRequest", &payload);

    let signer = NativeSigner::from_seed(&TEST_SEED);
    let signed_tx = native(signer)
        .chain_id("morpheum-mainnet-1")
        .add_message(msg)
        .sign()
        .await
        .expect("bank transfer signing failed");

    // Verify payload survived the roundtrip
    let decoded: Payload =
        serde_json::from_slice(&signed_tx.tx.body.as_ref().unwrap().messages[0].value)
            .expect("decode");
    assert_eq!(
        decoded.get("amount").and_then(|v| v.as_str()),
        Some("500000")
    );

    // Cryptographic verification
    let verified = verify_signed_tx(&signed_tx, "morpheum-mainnet-1", 0, &[], now_secs())
        .expect("verification failed");
    assert_eq!(verified.wallet_type, WalletType::Native);
}

// ==================== SIGN → VERIFY ROUNDTRIPS (ALL CURVES) ====================

/// NativeSigner (ed25519) → full cryptographic roundtrip.
#[tokio::test]
async fn test_native_sign_verify_roundtrip() {
    let signer = NativeSigner::from_seed(&TEST_SEED);
    let nonce_provider = TestNonceProvider { monotonic: 1 };

    let msg = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![1, 2, 3, 4],
    };

    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .memo("roundtrip-native")
        .with_nonce_provider(nonce_provider)
        .add_message(msg)
        .sign()
        .await
        .expect("native signing failed");

    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("native verification failed");

    assert_eq!(verified.account_ids.len(), 1);
    assert_eq!(verified.wallet_type, WalletType::Native);
    assert_eq!(verified.sign_mode, SignMode::Ed25519);
    assert!(verified.trading_key_claim.is_none());
    assert_eq!(verified.body.memo, "roundtrip-native");
    assert_eq!(verified.body.messages.len(), 1);
    assert_eq!(verified.nonce.as_ref().unwrap().monotonic, 1);
}

/// AgentSigner + TradingKeyClaim → roundtrip → claim extracted and verified.
///
/// This test exercises the full agent delegation flow end-to-end:
/// 1. Standards' OrderBuilder creates a CLOB order payload.
/// 2. Signing SDK's AgentSigner signs with an embedded TradingKeyClaim.
/// 3. `verify_signed_tx` verifies the signature and extracts the claim.
#[tokio::test]
async fn test_agent_claim_roundtrip() {
    let (signer, claim) = create_agent_with_matching_claim(&[99u8; 32]);

    // Build a CLOB order via standards
    let payload = OrderBuilder::new(
        "did:agent:alpha-v3",
        "did:agent:alpha-v3",
        "1000",
        "9999999999",
        "1",
        OrderSide::Sell,
        OrderType::Market,
        "500",
    )
    .build()
    .expect("OrderBuilder");

    let msg = payload_to_any(
        "type.googleapis.com/clob.v1.MsgSubmitOrderRequest",
        &payload,
    );

    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .memo("Agent with claim")
        .with_trading_key_claim(claim.clone())
        .add_message(msg)
        .sign()
        .await
        .expect("agent signing failed");

    // Verify the claim was embedded in signer_info
    let signer_info = &signed_tx.tx.auth_info.as_ref().unwrap().signer_infos[0];
    assert!(
        signer_info.signing_options.is_some(),
        "TradingKeyClaim should be embedded in signing_options"
    );

    // Full cryptographic verification + claim extraction
    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("agent verification failed");

    assert_eq!(verified.account_ids.len(), 1);
    // Ed25519 key → verifier infers Native (agent identity is in the claim)
    assert_eq!(verified.wallet_type, WalletType::Native);
    assert_eq!(verified.sign_mode, SignMode::Ed25519);

    // Verify the TradingKeyClaim was extracted correctly
    let extracted = verified
        .trading_key_claim
        .expect("TradingKeyClaim should be extracted from verified tx");
    assert_eq!(extracted.permissions, claim.permissions);
    assert_eq!(extracted.max_daily_usd, claim.max_daily_usd);
    assert_eq!(extracted.nonce_sub_range_start, claim.nonce_sub_range_start);
    assert_eq!(extracted.nonce_sub_range_end, claim.nonce_sub_range_end);
}

/// EvmSigner (secp256k1) → roundtrip verification.
#[tokio::test]
async fn test_evm_sign_verify_roundtrip() {
    let signer = EvmSigner::from_seed(&TEST_SEED);
    let payload = bank_transfer_payload("0xABCDabcd", "0xDEADdead", "1000");
    let msg = payload_to_any("type.googleapis.com/bank.v1.MsgTransferRequest", &payload);

    let signed_tx = evm(signer)
        .chain_id("morpheum-test-1")
        .memo("EVM roundtrip")
        .add_message(msg)
        .sign()
        .await
        .expect("EVM signing failed");

    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("EVM verification failed");

    assert_eq!(verified.account_ids.len(), 1);
    assert_eq!(verified.wallet_type, WalletType::Evm);
    assert!(matches!(
        verified.sign_mode,
        SignMode::Secp256k1 | SignMode::EcdsaLegacy | SignMode::Keccak256
    ));
}

/// SolanaSigner (ed25519) → roundtrip verification.
#[tokio::test]
async fn test_solana_sign_verify_roundtrip() {
    let signer = SolanaSigner::from_seed(&TEST_SEED);
    let msg = Any {
        type_url: "type.googleapis.com/test.v1.SolanaMsg".to_string(),
        value: vec![0xAA, 0xBB],
    };

    let signed_tx = solana(signer)
        .chain_id("morpheum-test-1")
        .add_message(msg)
        .sign()
        .await
        .expect("Solana signing failed");

    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("Solana verification failed");

    assert_eq!(verified.account_ids.len(), 1);
    // Solana uses ed25519, so verifier infers Native at the key level
    assert_eq!(verified.sign_mode, SignMode::Ed25519);
}

// ==================== EIP-712 TYPED DATA → CRYPTO SIGN/VERIFY ====================

/// Full EIP-712 pipeline: registry → typed data → digest → sign → verify.
///
/// Tests the complete standards registry + cryptogram-crypto pipeline independently
/// of the signing SDK. Validates that these two crates produce correct, interoperable output.
#[tokio::test]
async fn test_eip712_digest_sign_verify_via_standards() {
    // 1. Build an ActionRegistry with all registered actions
    let mut registry = ActionRegistry::new();
    register_all_actions(&mut registry);

    // 2. Build a bank::transfer payload
    let mut payload: Payload = serde_json::Map::new();
    payload.insert("action".into(), serde_json::json!("bank::transfer"));
    payload.insert("owner".into(), serde_json::json!("morpheum1test"));
    payload.insert("toAddress".into(), serde_json::json!("morpheum1dest"));
    payload.insert("amount".into(), serde_json::json!("1000000"));
    payload.insert("assetIndex".into(), serde_json::json!(0u64));
    payload.insert("nonce".into(), serde_json::json!(1u64));
    payload.insert("deadline".into(), serde_json::json!(9_999_999_999i64));

    // 3. Create EIP-712 typed data via registry
    let domain = domain_testnet();
    let typed_data =
        create_typed_data_from_payload(&registry, &payload, domain).expect("typed data creation");

    // 4. Compute digest (B256 = FixedBytes<32>)
    let digest: [u8; 32] = typed_data.digest().0;

    // 5. Sign digest with ed25519 via cryptogram-crypto
    let secret_key = TEST_SEED;
    let signature = ed25519_sign(&digest, &secret_key).expect("ed25519 sign failed");

    // 6. Verify signature with ed25519 via cryptogram-crypto
    let public_key = ed25519_public_key(&secret_key);
    let valid = ed25519_verify(&digest, &signature, &public_key).expect("ed25519 verify failed");
    assert!(valid, "ed25519 signature should verify correctly");
}

// ==================== MULTI-MESSAGE CROSS-CRATE ====================

/// Multiple standards-built payloads in a single signing SDK transaction.
///
/// Verifies that heterogeneous messages (bank transfer + CLOB order) can be
/// batched into one transaction and survive the sign → verify roundtrip.
#[tokio::test]
async fn test_multiple_standards_messages_in_single_tx() {
    let transfer_payload = bank_transfer_payload("morpheum1a", "morpheum1b", "100");
    let order_payload = OrderBuilder::new(
        "morpheum1a",
        "morpheum1a",
        "1",
        "9999999999",
        "2",
        OrderSide::Buy,
        OrderType::Market,
        "50",
    )
    .build()
    .expect("order payload");

    let msg1 = payload_to_any(
        "type.googleapis.com/bank.v1.MsgTransferRequest",
        &transfer_payload,
    );
    let msg2 = payload_to_any(
        "type.googleapis.com/clob.v1.MsgSubmitOrderRequest",
        &order_payload,
    );

    let signer = NativeSigner::from_seed(&TEST_SEED);
    let signed_tx = native(signer)
        .chain_id("morpheum-test-1")
        .memo("Multi-message cross-crate")
        .add_message(msg1)
        .add_message(msg2)
        .sign()
        .await
        .expect("multi-message signing failed");

    let body = signed_tx.tx.body.as_ref().unwrap();
    assert_eq!(body.messages.len(), 2);
    assert_eq!(
        body.messages[0].type_url,
        "type.googleapis.com/bank.v1.MsgTransferRequest"
    );
    assert_eq!(
        body.messages[1].type_url,
        "type.googleapis.com/clob.v1.MsgSubmitOrderRequest"
    );

    // Verify roundtrip
    let verified = verify_signed_tx(&signed_tx, "morpheum-test-1", 0, &[], now_secs())
        .expect("multi-message verification failed");
    assert_eq!(verified.body.messages.len(), 2);
    assert_eq!(verified.account_ids.len(), 1);
}

// ==================== EXPLICIT CRYPTOGRAM-CRYPTO DEPENDENCY ====================

/// Uses the explicit `cryptogram_crypto` re-export from `morpheum-signing-core`
/// (not the bridge module) to sign and verify a digest. This validates that the
/// direct dependency declared in Cargo.toml resolves correctly and that low-level
/// crypto primitives are accessible without intermediate wrappers.
#[tokio::test]
async fn test_explicit_cryptogram_crypto_reexport() {
    // Access cryptogram-crypto directly through the new crate-level re-export
    use morpheum_signing_core::cryptogram_crypto;

    // Use a fixed 32-byte digest as test vector (avoids extra sha2 dev-dep)
    let digest: [u8; 32] = [0xAB; 32];

    // Sign using the explicit re-export path
    let signature = cryptogram_crypto::ed25519_sign(&digest, &TEST_SEED)
        .expect("ed25519_sign via explicit re-export");

    // Verify using the explicit re-export path
    let pubkey = cryptogram_crypto::ed25519_public_key(&TEST_SEED);
    let valid = cryptogram_crypto::ed25519_verify(&digest, &signature, &pubkey)
        .expect("ed25519_verify via explicit re-export");
    assert!(
        valid,
        "signature must verify via explicit cryptogram-crypto re-export"
    );

    // Also verify via the universal dispatcher to exercise the full API surface
    let sig_for_universal = Eip712Signature {
        signer: hex::encode(pubkey),
        signature: hex::encode(signature),
        sig_type: Some(SigType::Ed25519),
        timestamp: None,
    };
    let universal_valid = cryptogram_crypto::verify_single_from_digest(
        &digest,
        &[sig_for_universal],
        SigType::Ed25519,
        1,
    )
    .expect("universal verify via explicit re-export");
    assert!(universal_valid, "universal verifier must agree");
}

// ==================== DETERMINISM ====================

/// Same standards payload + same signer seed → identical SignedTx.
///
/// Validates that neither the standards payload construction nor the signing
/// pipeline introduces non-determinism.
#[tokio::test]
async fn test_cross_crate_deterministic_signing() {
    let payload = OrderBuilder::new(
        "morpheum1test",
        "morpheum1test",
        "1",
        "9999999999",
        "1",
        OrderSide::Buy,
        OrderType::Market,
        "100",
    )
    .build()
    .expect("order payload");

    let msg = || {
        payload_to_any(
            "type.googleapis.com/clob.v1.MsgSubmitOrderRequest",
            &payload,
        )
    };

    let nonce1 = TestNonceProvider { monotonic: 42 };
    let nonce2 = TestNonceProvider { monotonic: 42 };

    let tx1 = native(NativeSigner::from_seed(&TEST_SEED))
        .chain_id("morpheum-test-1")
        .with_nonce_provider(nonce1)
        .add_message(msg())
        .sign()
        .await
        .expect("sign 1");

    let tx2 = native(NativeSigner::from_seed(&TEST_SEED))
        .chain_id("morpheum-test-1")
        .with_nonce_provider(nonce2)
        .add_message(msg())
        .sign()
        .await
        .expect("sign 2");

    assert_eq!(tx1.raw_bytes, tx2.raw_bytes, "same inputs → same output");
    assert_eq!(tx1.tx.signatures, tx2.tx.signatures);
}
