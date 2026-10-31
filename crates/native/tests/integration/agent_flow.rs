//! Agent signer flow tests (TradingKey + VC claim with isolated nonce sub-range).
//!
//! Verifies the full agent signing flow using `AgentSigner` and the `agent()` builder.

use super::common::*;
use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
use morpheum_signing_native::prelude::*;
use morpheum_signing_native::sign_doc_signing_bytes;
use prost::Message;

#[tokio::test]
pub async fn test_agent_signing_flow() {
    let trading_key_seed = [99u8; 32];
    let agent_id = test_account_id();
    let trading_key_claim = test_trading_key_claim();

    let signer = AgentSigner::new(&trading_key_seed, agent_id, Some(trading_key_claim.clone()));

    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![],
    };

    let sub_start = trading_key_claim.nonce_sub_range_start;
    let sub_end = trading_key_claim.nonce_sub_range_end;

    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .with_genesis_hash([0x5A_u8; 32])
        .memo("Agent integration test with TradingKey + VC")
        .add_message(market_any)
        .with_trading_key_claim(trading_key_claim)
        .sign()
        .await
        .expect("Agent signing failed");

    assert!(
        !signed_tx.raw_bytes.is_empty(),
        "Signed tx should not be empty"
    );
    let sub = signed_tx
        .tx
        .nonce
        .as_ref()
        .expect("signed tx carries a nonce")
        .sub;
    assert!(
        sub_start > 0,
        "the claim range must exclude the unset sub 0"
    );
    assert!(
        (sub_start..sub_end).contains(&sub),
        "signed nonce.sub {sub} must sit inside the claim range [{sub_start}, {sub_end})"
    );
    assert_eq!(
        signed_tx.tx.body.as_ref().unwrap().memo,
        "Agent integration test with TradingKey + VC"
    );
    assert_eq!(signed_tx.tx.body.as_ref().unwrap().messages.len(), 1);
}

#[tokio::test]
pub async fn test_agent_without_claim() {
    let signer = AgentSigner::new(&[77u8; 32], test_account_id(), None);

    let msg = Any {
        type_url: "type.googleapis.com/test.v1.Msg".to_string(),
        value: vec![],
    };

    let signed_tx = agent(signer)
        .chain_id("morpheum-test-1")
        .with_genesis_hash([0x5A_u8; 32])
        .add_message(msg)
        .sign()
        .await
        .expect("Agent without claim should succeed");

    assert!(!signed_tx.raw_bytes.is_empty());
    assert_eq!(
        signed_tx.tx.nonce.as_ref().map(|nonce| nonce.sub),
        Some(0),
        "an agent sign with no claim and no provider keeps nonce.sub at 0"
    );
}

#[tokio::test]
pub async fn test_agent_signs_the_claim_stored_on_the_signer() {
    let claim = test_trading_key_claim();
    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(claim.clone()));

    let signed = agent(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(Any {
            type_url: "type.googleapis.com/clob.v1.MsgPlaceOrder".to_string(),
            value: vec![1, 2, 3],
        })
        .sign()
        .await
        .expect("an agent signer with a stored claim signs when the builder sets none");

    assert_eq!(
        embedded_claim(&signed),
        Some(claim.clone()),
        "the signed tx carries the claim stored on the signer"
    );
    assert_eq!(
        signed.tx.nonce.as_ref().map(|nonce| nonce.sub),
        Some(claim.nonce_sub_range_start),
        "omitting a builder claim stamps nonce.sub from the claim stored on the signer"
    );
}

#[tokio::test]
pub async fn test_tx_builder_new_signs_the_stored_claim_on_the_no_provider_path() {
    let claim = test_trading_key_claim();
    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(claim.clone()));

    let signed = TxBuilder::new(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(place_order())
        .sign()
        .await
        .expect("TxBuilder::new signs when the caller sets no claim");

    assert_eq!(
        embedded_claim(&signed),
        Some(claim.clone()),
        "TxBuilder::new attaches the claim stored on the signer"
    );
    assert_eq!(
        signed.tx.nonce.as_ref().map(|nonce| nonce.sub),
        Some(claim.nonce_sub_range_start),
        "the no-provider path stamps nonce.sub from the stored claim start"
    );
}

#[tokio::test]
pub async fn test_agent_and_tx_builder_new_sign_the_same_bytes() {
    let claim = test_trading_key_claim();
    let seed = [99u8; 32];

    let via_agent = agent(AgentSigner::new(
        &seed,
        test_account_id(),
        Some(claim.clone()),
    ))
    .chain_id(ORDER_CHAIN_ID)
    .with_genesis_hash(ORDER_GENESIS_HASH)
    .account_number(ORDER_ACCOUNT_NUMBER)
    .add_message(place_order())
    .sign()
    .await
    .expect("agent() signs the stored claim");

    let via_new = TxBuilder::new(AgentSigner::new(&seed, test_account_id(), Some(claim)))
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(place_order())
        .sign()
        .await
        .expect("TxBuilder::new signs the stored claim");

    assert_eq!(
        embedded_claim(&via_new),
        embedded_claim(&via_agent),
        "both entry points attach the same claim"
    );
    assert_eq!(
        via_new.tx.nonce.as_ref().map(|nonce| nonce.sub),
        via_agent.tx.nonce.as_ref().map(|nonce| nonce.sub),
        "both entry points stamp the same nonce.sub"
    );
    assert_eq!(
        via_new.raw_bytes, via_agent.raw_bytes,
        "both entry points produce the same signed bytes"
    );
}

fn embedded_claim(signed: &SignedTx) -> Option<TradingKeyClaim> {
    let tx_raw = signed.tx_raw().expect("builder returns TxRaw");
    let auth = AuthInfo::decode(tx_raw.auth_info_bytes.as_slice()).expect("auth info decodes");
    let signer_info = auth.signer_infos.first().expect("one signer info");
    match signer_info.signing_options.as_ref() {
        Some(opts) => {
            TradingKeyClaim::decode_from_signing_options(opts).expect("embedded claim decodes")
        }
        None => None,
    }
}

fn claim_bytes_differ(stored: &TradingKeyClaim) -> TradingKeyClaim {
    let attached = TradingKeyClaim {
        nonce_sub_range_start: stored.nonce_sub_range_start + 1,
        ..stored.clone()
    };
    assert!(
        attached.nonce_sub_range_start < attached.nonce_sub_range_end,
        "the disagreeing claim stays inside a non-empty range"
    );
    assert_ne!(
        stored.encode_to_vec(),
        attached.encode_to_vec(),
        "the caller passes different claim bytes"
    );
    attached
}

fn place_order() -> Any {
    Any {
        type_url: "type.googleapis.com/clob.v1.MsgPlaceOrder".to_string(),
        value: vec![1, 2, 3],
    }
}

struct CountingNonce {
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    sub: u32,
}

#[async_trait::async_trait]
impl NonceProvider for CountingNonce {
    async fn next_nonce(&self, _account_id: &AccountId) -> Result<Nonce, SigningError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(Nonce {
            monotonic: 1,
            ts_ms: 2,
            sub: self.sub,
        })
    }
}

fn assert_claim_disagreement_refused(
    result: Result<SignedTx, SigningError>,
    calls: &std::sync::atomic::AtomicUsize,
) {
    match result {
        Ok(signed) => panic!(
            "a disagreeing claim must not sign, got {} signature bytes",
            signed
                .tx
                .signatures
                .first()
                .map(|sig| sig.len())
                .unwrap_or(0)
        ),
        Err(SigningError::InvalidClaim(msg)) if msg.contains("disagrees") => {}
        Err(err) => panic!("a disagreeing claim must be an invalid claim, got {err}"),
    }
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "a refused claim must not ask the nonce provider"
    );
}

#[tokio::test]
pub async fn test_sign_refuses_a_trading_key_claim_that_disagrees_with_the_signer() {
    let stored = test_trading_key_claim();
    let attached = claim_bytes_differ(&stored);
    let sub = attached.nonce_sub_range_start;
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(stored));

    let result = agent(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(place_order())
        .with_trading_key_claim(attached)
        .with_nonce_provider(CountingNonce {
            calls: std::sync::Arc::clone(&calls),
            sub,
        })
        .sign()
        .await;

    assert_claim_disagreement_refused(result, &calls);
}

#[tokio::test]
pub async fn test_tx_builder_new_refuses_a_trading_key_claim_that_disagrees() {
    let stored = test_trading_key_claim();
    let attached = claim_bytes_differ(&stored);
    let sub = attached.nonce_sub_range_start;
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(stored));

    let result = TxBuilder::new(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(place_order())
        .with_trading_key_claim(attached)
        .with_nonce_provider(CountingNonce {
            calls: std::sync::Arc::clone(&calls),
            sub,
        })
        .sign()
        .await;

    assert_claim_disagreement_refused(result, &calls);
}

#[tokio::test]
pub async fn test_identical_trading_key_claim_bytes_still_sign() {
    let stored = test_trading_key_claim();
    let passed = stored.clone();
    assert_eq!(
        stored.encode_to_vec(),
        passed.encode_to_vec(),
        "the caller passes the stored claim's bytes"
    );

    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), Some(stored.clone()));
    let signed = agent(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(place_order())
        .with_trading_key_claim(passed)
        .sign()
        .await
        .expect("the same claim bytes still sign");

    let signature = signed
        .tx
        .signatures
        .first()
        .expect("the same claim bytes produce a signature");
    assert!(
        !signature.is_empty(),
        "the same claim bytes produce a signature"
    );
    assert_eq!(
        embedded_claim(&signed),
        Some(stored),
        "the signed tx carries those claim bytes"
    );
}

#[tokio::test]
pub async fn test_agent_different_seeds_produce_different_signatures() {
    let signer1 = AgentSigner::new(&[1u8; 32], test_account_id(), None);
    let signer2 = AgentSigner::new(&[2u8; 32], test_account_id(), None);

    let msg = || Any {
        type_url: "type.googleapis.com/test.v1.Msg".to_string(),
        value: vec![42],
    };

    let tx1 = agent(signer1)
        .chain_id("morpheum-test-1")
        .with_genesis_hash([0x5A_u8; 32])
        .add_message(msg())
        .sign()
        .await
        .expect("Sign 1 failed");

    let tx2 = agent(signer2)
        .chain_id("morpheum-test-1")
        .with_genesis_hash([0x5A_u8; 32])
        .add_message(msg())
        .sign()
        .await
        .expect("Sign 2 failed");

    assert_ne!(
        tx1.tx.signatures, tx2.tx.signatures,
        "Different keys must produce different sigs"
    );
}

const ORDER_CHAIN_ID: &str = "morpheum-test-1";
const ORDER_GENESIS_HASH: [u8; 32] = [0x5A; 32];
const ORDER_ACCOUNT_NUMBER: u64 = 7;

async fn sign_order(signer: AgentSigner, claim: TradingKeyClaim) -> SignedTx {
    agent(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .account_number(ORDER_ACCOUNT_NUMBER)
        .add_message(Any {
            type_url: "type.googleapis.com/clob.v1.MsgPlaceOrder".to_string(),
            value: vec![1, 2, 3],
        })
        .with_trading_key_claim(claim)
        .sign()
        .await
        .expect("agent order signs")
}

#[tokio::test]
pub async fn test_agent_order_verifies_under_trading_key_and_binds_claim() {
    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), None);
    let claim = TradingKeyClaim {
        subject: AccountId([0xAB; 32]),
        ..test_trading_key_claim()
    };

    let signed = sign_order(signer.clone(), claim.clone()).await;
    let resigned = sign_order(signer.clone(), claim.clone()).await;
    assert_eq!(
        signed.raw_bytes, resigned.raw_bytes,
        "a retried order must re-sign to the same bytes"
    );

    let tx_raw = signed.tx_raw().expect("builder returns TxRaw").clone();
    assert_eq!(
        tx_raw.signatures, signed.tx.signatures,
        "the broadcast signature is the signature on the tx"
    );

    let preimage = sign_doc_signing_bytes(
        tx_raw.body_bytes.clone(),
        tx_raw.auth_info_bytes.clone(),
        ORDER_CHAIN_ID,
        ORDER_ACCOUNT_NUMBER,
        ORDER_GENESIS_HASH.to_vec(),
        signed.tx.nonce,
    );

    let PublicKey::Ed25519(trading_key) = signer.public_key() else {
        panic!("AgentSigner exposes an ed25519 TradingKey");
    };
    let trading_key = VerifyingKey::from_bytes(&trading_key).expect("valid ed25519 point");
    let signature = Ed25519Signature::from_slice(tx_raw.signatures.first().expect("one signature"))
        .expect("raw 64-byte RFC 8032 signature");
    trading_key
        .verify(&preimage, &signature)
        .expect("signature verifies under public_key() over the canonical SignDoc");

    let sub = signed
        .tx
        .nonce
        .as_ref()
        .expect("signed order carries a nonce")
        .sub;
    assert!(
        claim.nonce_sub_range_start > 0,
        "the claim range must exclude the unset sub 0"
    );
    assert!(
        (claim.nonce_sub_range_start..claim.nonce_sub_range_end).contains(&sub),
        "signed nonce.sub {sub} must sit inside the claim range [{}, {})",
        claim.nonce_sub_range_start,
        claim.nonce_sub_range_end,
    );

    let auth = AuthInfo::decode(tx_raw.auth_info_bytes.as_slice()).expect("auth info decodes");
    let signer_info = auth.signer_infos.first().expect("one signer info");
    let embedded = TradingKeyClaim::decode_from_signing_options(
        signer_info
            .signing_options
            .as_ref()
            .expect("claim is embedded"),
    )
    .expect("embedded claim decodes");
    assert_eq!(
        embedded,
        Some(claim),
        "the signed tx carries the exact claim"
    );
    let expected_address = "ab".repeat(32);
    assert_eq!(
        signer_info.trading_key_address.as_deref(),
        Some(expected_address.as_str()),
        "trading_key_address is the lowercase hex of the claim subject"
    );
}

struct SuppliedSub(u32);

#[async_trait::async_trait]
impl NonceProvider for SuppliedSub {
    async fn next_nonce(&self, _account_id: &AccountId) -> Result<Nonce, SigningError> {
        Ok(Nonce {
            monotonic: 4,
            ts_ms: 5,
            sub: self.0,
        })
    }
}

#[tokio::test]
pub async fn test_agent_claim_keeps_a_supplied_sub() {
    let claim = test_trading_key_claim();
    let supplied = claim.nonce_sub_range_start + 1;
    assert!(
        supplied < claim.nonce_sub_range_end,
        "the supplied sub must itself sit inside the claim range"
    );

    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), None);
    let signed = agent(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .add_message(Any {
            type_url: "type.googleapis.com/clob.v1.MsgPlaceOrder".to_string(),
            value: vec![1],
        })
        .with_trading_key_claim(claim)
        .with_nonce_provider(SuppliedSub(supplied))
        .sign()
        .await
        .expect("agent order with a nonce provider signs");

    assert_eq!(
        signed.tx.nonce.as_ref().map(|nonce| nonce.sub),
        Some(supplied),
        "a provider sub is the signed sub"
    );
}

async fn sign_order_with_manual_sub(
    claim: TradingKeyClaim,
    sub: u32,
) -> Result<SignedTx, SigningError> {
    let signer = AgentSigner::new(&[99u8; 32], test_account_id(), None);
    agent(signer)
        .chain_id(ORDER_CHAIN_ID)
        .with_genesis_hash(ORDER_GENESIS_HASH)
        .add_message(Any {
            type_url: "type.googleapis.com/clob.v1.MsgPlaceOrder".to_string(),
            value: vec![1],
        })
        .with_trading_key_claim(claim)
        .with_nonce(Nonce {
            monotonic: 1,
            ts_ms: 2,
            sub,
        })
        .sign()
        .await
}

fn assert_signed_sub(result: Result<SignedTx, SigningError>, sub: u32) {
    let signed = result.unwrap_or_else(|err| {
        panic!("nonce.sub {sub} is inside the claim range and must sign, got {err}")
    });
    assert_eq!(
        signed.tx.nonce.as_ref().map(|nonce| nonce.sub),
        Some(sub),
        "the manual sub is the signed sub"
    );
}

fn assert_sub_rejected(result: Result<SignedTx, SigningError>, sub: u32) {
    match result {
        Ok(_) => panic!("nonce.sub {sub} is outside the claim range and must not sign"),
        Err(SigningError::InvalidClaim(msg)) if msg.contains("outside") => {}
        Err(err) => panic!("nonce.sub {sub} must fail as an invalid claim, got {err}"),
    }
}

#[tokio::test]
pub async fn test_agent_sign_rejects_nonce_sub_outside_claim_range() {
    let claim = test_trading_key_claim();
    let start = claim.nonce_sub_range_start;
    let end = claim.nonce_sub_range_end;
    assert!(start > 0, "the fixture range must have a sub below start");
    assert!(
        end > start + 1,
        "the fixture range must contain a last valid sub"
    );

    assert_signed_sub(
        sign_order_with_manual_sub(claim.clone(), start).await,
        start,
    );
    assert_signed_sub(
        sign_order_with_manual_sub(claim.clone(), end - 1).await,
        end - 1,
    );
    assert_sub_rejected(
        sign_order_with_manual_sub(claim.clone(), start - 1).await,
        start - 1,
    );
    assert_sub_rejected(sign_order_with_manual_sub(claim, end).await, end);
}
