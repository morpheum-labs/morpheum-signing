//! Signer-less signing-preimage assembly.
//!
//! Builds the `TxBody`, `AuthInfo` and `SignDoc` a transaction's signature will
//! cover, for a caller that holds a raw public key rather than a connected
//! [`Signer`](crate::signer::Signer). [`TxBuilder`](crate::builder::TxBuilder)
//! is the counterpart for callers that do have one.
//!
//! # Why this lives in `core` and not in the wasm crate
//!
//! It used to live in `morpheum-signing-wasm`, which is
//! `#![cfg(target_arch = "wasm32")]` — an empty crate on the host, so nothing
//! any host CI runs could compile it, let alone test it. That is how an arity
//! break reached `main` once already. Preimage assembly decides what a
//! signature covers; it is the last thing that should be invisible to the test
//! runner.
//!
//! Here it is ordinary Rust: host-compiled, host-tested, and pinned by a golden
//! vector below. The wasm binding keeps only what genuinely needs a JS
//! boundary — decoding the request object and marshalling the result back.
//!
//! # Why a struct rather than parameters
//!
//! The wasm entry point took ten positional parameters, and this crate's
//! defining defect was a ten-parameter call made with eight arguments: the
//! trailing `genesis_hash` and `nonce` silently defaulted, so every transaction
//! shipped a nonce no signature covered. A missing field on a named struct is a
//! compile error; a missing trailing argument is a security downgrade. Rust
//! also has no default-argument rule to rescue the positional form, which is
//! why `clippy::too_many_arguments` fires on it at 10/7.

use prost::Message;
use sha2::{Digest, Sha256};

use morpheum_primitives::pb::tx::v1::{self as tx, AuthInfo, ModeInfo, Nonce, SignerInfo, TxBody};
use morpheum_primitives::tx::sign_doc_signing_bytes;

use crate::proto::Any;

/// Proto `type_url` for a 20-byte EVM address or a SEC1 secp256k1 key.
const SECP256K1_PUBKEY_TYPE_URL: &str = "/cosmos.crypto.secp256k1.PubKey";
/// Proto `type_url` for a 32-byte Ed25519 key.
const ED25519_PUBKEY_TYPE_URL: &str = "/cosmos.crypto.ed25519.PubKey";
/// `ChainType::Ethereum` on the wire. Anything else signs with Ed25519 keys.
const CHAIN_TYPE_ETHEREUM: i32 = 1;

/// Everything the preimage covers, named.
///
/// Every field is required. `memo` and `genesis_hash` are `Option` because
/// absent and empty are the same statement *for those two only* — an omitted
/// memo and an empty memo encode identically, and an absent genesis hash is the
/// pre-fork unbound posture. `nonce` is deliberately **not** optional: omitting
/// it is what produced a signature that did not cover the replay-protection
/// field the transaction shipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignDocRequest {
    /// Proto type URL of the single message this transaction carries.
    pub type_url: String,
    /// Pre-encoded protobuf bytes of that message.
    pub msg_bytes: Vec<u8>,
    /// Raw signer key bytes — a 20-byte EVM address, or a 32-byte Ed25519 key.
    ///
    /// Bytes rather than a hex string: decoding is the caller's boundary
    /// concern, and a type that cannot hold "0xzz" is one fewer error path
    /// here.
    pub signer_key: Vec<u8>,
    /// `ChainType` discriminant, which selects the public-key type URL.
    pub chain_type: i32,
    /// `SignMode` discriminant recorded in `ModeInfo`.
    pub sign_mode: i32,
    /// Chain identifier bound into the preimage.
    pub chain_id: String,
    /// Optional transaction memo.
    pub memo: Option<String>,
    /// Account number bound into the preimage.
    pub account_number: u64,
    /// Genesis hash (Phase M3 — audit `O20` / `C12`) binding the signature to
    /// one chain instance, so it cannot be replayed onto another sharing a
    /// `chain_id`. `None` is the pre-fork unbound posture.
    pub genesis_hash: Option<Vec<u8>>,
    /// The nonce this preimage binds. Returned in [`SignDocParts::nonce`] so
    /// the caller stamps the value the signature actually covered.
    pub nonce: Nonce,
}

/// The assembled preimage and the encodings a caller needs to build the
/// matching transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignDocParts {
    /// Proto-encoded `SignDoc` — the exact bytes to sign.
    pub sign_doc_bytes: Vec<u8>,
    /// `SHA-256(sign_doc_bytes)`.
    pub sign_doc_hash: [u8; 32],
    /// Proto-encoded `TxBody`.
    pub body_bytes: Vec<u8>,
    /// Proto-encoded `AuthInfo`.
    pub auth_info_bytes: Vec<u8>,
    /// Re-encoded from the `Nonce` this preimage bound.
    ///
    /// Derived from what was bound rather than echoing what was passed, so a
    /// caller stamping this onto `Tx.nonce` cannot ship a value the signature
    /// does not cover.
    pub nonce_bytes: Vec<u8>,
}

/// Assembles the signing preimage described by `request`.
///
/// Total: every input is already typed, so there is no failure mode left to
/// report. Hex decoding and proto decoding happen at the caller's boundary,
/// which is where the malformed input actually arrives.
#[must_use]
pub fn build_sign_doc(request: &SignDocRequest) -> SignDocParts {
    let key_type_url = if request.chain_type == CHAIN_TYPE_ETHEREUM {
        SECP256K1_PUBKEY_TYPE_URL
    } else {
        ED25519_PUBKEY_TYPE_URL
    };

    // Every field is written out explicitly, and deliberately without
    // `..Default::default()`. This struct is a *signing preimage*: whatever it
    // encodes to is what the signature covers, so the caller must send a
    // `TxBody` that encodes identically or verification fails. A spread would
    // let a newly added proto field default silently on this side while the
    // wire carried something else — a signature over one transaction attached
    // to another. Failing to compile when the proto grows is the safer
    // outcome: it forces the value to be chosen rather than assumed. (That is
    // exactly what happened with `tx_class` and `urgent`.)
    let body = TxBody {
        messages: vec![Any {
            type_url: request.type_url.clone(),
            value: request.msg_bytes.clone(),
        }],
        memo: request.memo.clone().unwrap_or_default(),
        timeout_timestamp: None,
        priority_tip: String::new(),
        tx_class: crate::tx_class::TxClass::Standard.to_wire(),
        urgent: false,
    };

    let signer_info = SignerInfo {
        public_key: Some(Any {
            type_url: key_type_url.into(),
            value: request.signer_key.clone(),
        }),
        mode_info: Some(ModeInfo {
            sum: Some(tx::mode_info::Sum::Single(tx::mode_info::Single {
                mode: request.sign_mode,
            })),
        }),
        chain_type: request.chain_type,
        ..Default::default()
    };

    let auth_info = AuthInfo {
        signer_infos: vec![signer_info],
        gas_limit: 0,
    };

    let body_bytes = body.encode_to_vec();
    let auth_info_bytes = auth_info.encode_to_vec();

    // Assembled through the preimage SSOT in `morpheum-primitives` so browser
    // and native signers produce byte-identical preimages. There is no other
    // way to compute these bytes, which is the gate closing audit row C12 /
    // gap O20: no call site can silently drop a binding.
    let sign_doc_bytes = sign_doc_signing_bytes(
        body_bytes.clone(),
        auth_info_bytes.clone(),
        &request.chain_id,
        request.account_number,
        request.genesis_hash.clone().unwrap_or_default(),
        Some(request.nonce),
    );

    SignDocParts {
        sign_doc_hash: Sha256::digest(&sign_doc_bytes).into(),
        sign_doc_bytes,
        body_bytes,
        auth_info_bytes,
        nonce_bytes: request.nonce.encode_to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The request the golden vector below was captured from.
    fn golden_request() -> SignDocRequest {
        SignDocRequest {
            type_url: "/bucket.v1.MsgCreateBucketRequest".into(),
            msg_bytes: vec![0x0a, 0x06, 0x67, 0x6f, 0x6c, 0x64, 0x65, 0x6e],
            signer_key: vec![0xab; 20],
            chain_type: 1,
            sign_mode: 7,
            chain_id: "morm-dev-1".into(),
            memo: Some("golden-memo".into()),
            account_number: 9,
            genesis_hash: Some((0u8..32).collect()),
            // Decoded from the exact `Nonce` bytes the golden capture passed:
            // 08 03 | 10 80 d8 d0 ca 06 | 18 05.
            nonce: Nonce {
                monotonic: 3,
                ts_ms: 1_767_123_968,
                sub: 5,
            },
        }
    }

    /// **Golden vector.** Pins the exact bytes a signature covers.
    ///
    /// Captured from the shipped `pkg-node` binding *before* this assembly
    /// moved out of the wasm crate, by calling `buildSignDocBytes` from Node
    /// with these inputs. So it does not merely pin the current implementation
    /// against itself — it proves the move preserved the preimage byte for
    /// byte, and it fails on any future change to field order, tag numbers,
    /// default handling, or the `TxBody` / `AuthInfo` shape.
    ///
    /// A signature is only as meaningful as the bytes it covers. If these
    /// change without the verifiers changing in lockstep, every existing
    /// signature silently stops verifying — or worse, keeps verifying over
    /// something else.
    /// The exact bytes the shipped `pkg-node` binding produced for
    /// [`golden_request`], captured by calling `buildSignDocBytes` from Node
    /// **before** this assembly moved out of the wasm crate.
    ///
    /// Single unbroken literals on purpose. The first draft wrapped them across
    /// lines and stripped the whitespace back out, and the wrapping silently
    /// gained a byte — a golden vector transcribed by hand is only as good as
    /// the transcription, so there is nothing here to transcribe.
    const GOLDEN_SIGN_DOC_HASH: &str =
        "e2279894897d1b2c7f2621a6812f2baaf2ed617be42a33e40d4d3cdd787b0df7";
    const GOLDEN_BODY_BYTES: &str =
        "0a2d0a212f6275636b65742e76312e4d73674372656174654275636b65745265717565737412080a06676f6c64656e120b676f6c64656e2d6d656d6f";
    const GOLDEN_AUTH_INFO_BYTES: &str =
        "0a410a370a1f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b65791214abababababababababababababababababababab12040a0208071801";
    const GOLDEN_NONCE_BYTES: &str = "08031080d8d0ca061805";

    /// **Golden vector.** Pins the exact bytes a signature covers.
    ///
    /// Because the expected values come from the previous implementation rather
    /// than from this one, this does not merely pin the code against itself: it
    /// proves the move out of the wasm crate preserved the preimage byte for
    /// byte. It then fails on any future change to field order, tag numbers,
    /// default handling, or the `TxBody` / `AuthInfo` shape.
    ///
    /// A signature is only as meaningful as the bytes it covers. If these change
    /// without every verifier changing in lockstep, existing signatures stop
    /// verifying — or, worse, keep verifying over something else.
    #[test]
    fn preimage_matches_the_golden_vector() {
        let parts = build_sign_doc(&golden_request());

        assert_eq!(
            hex::encode(parts.sign_doc_hash),
            GOLDEN_SIGN_DOC_HASH,
            "SignDoc hash drift — the bytes a signature covers have changed",
        );
        assert_eq!(
            hex::encode(&parts.body_bytes),
            GOLDEN_BODY_BYTES,
            "TxBody encoding drift",
        );
        assert_eq!(
            hex::encode(&parts.auth_info_bytes),
            GOLDEN_AUTH_INFO_BYTES,
            "AuthInfo encoding drift",
        );
        assert_eq!(
            hex::encode(&parts.nonce_bytes),
            GOLDEN_NONCE_BYTES,
            "Nonce re-encoding drift",
        );
    }

    /// The returned nonce is the one that was bound, not a look-alike.
    ///
    /// This is the property that makes signed-vs-sent divergence
    /// unrepresentable: assembly has no other nonce to reach for.
    #[test]
    fn returned_nonce_re_encodes_the_bound_nonce() {
        let request = golden_request();
        let parts = build_sign_doc(&request);

        assert_eq!(
            Nonce::decode(parts.nonce_bytes.as_slice()).expect("re-encoded nonce must decode"),
            request.nonce,
        );
    }

    /// Binding a genesis hash changes the preimage, and `None` is not the same
    /// as `Some(vec![])`-that-encodes-to-nothing by accident.
    ///
    /// An unbound and a bound signature must not collide: if they did, the
    /// strict genesis fork would be unenforceable because the two rungs would
    /// be the same byte string.
    #[test]
    fn genesis_binding_changes_the_preimage() {
        let bound = build_sign_doc(&golden_request());

        let unbound = build_sign_doc(&SignDocRequest {
            genesis_hash: None,
            ..golden_request()
        });
        assert_ne!(
            bound.sign_doc_bytes, unbound.sign_doc_bytes,
            "binding a genesis hash must change the bytes a signature covers",
        );

        // `None` and an explicitly empty hash are the same posture — both mean
        // "binds no genesis" — so they must produce identical bytes. A verifier
        // builds its unbound rung from an empty slice; if these differed, a
        // client passing `Some(vec![])` would sign something no rung matches.
        let empty = build_sign_doc(&SignDocRequest {
            genesis_hash: Some(Vec::new()),
            ..golden_request()
        });
        assert_eq!(unbound.sign_doc_bytes, empty.sign_doc_bytes);
    }

    /// The public-key type URL follows `chain_type`, because the verifier
    /// parses the key by that URL: an EVM address announced as Ed25519 is
    /// rejected as a bad key length rather than as the wrong chain.
    #[test]
    fn chain_type_selects_the_public_key_type_url() {
        let evm = build_sign_doc(&golden_request());
        let other = build_sign_doc(&SignDocRequest {
            chain_type: 2,
            signer_key: vec![0xcd; 32],
            ..golden_request()
        });

        let decode = |bytes: &[u8]| {
            AuthInfo::decode(bytes)
                .expect("AuthInfo must decode")
                .signer_infos[0]
                .public_key
                .clone()
                .expect("public key must be present")
                .type_url
        };

        assert_eq!(decode(&evm.auth_info_bytes), SECP256K1_PUBKEY_TYPE_URL);
        assert_eq!(decode(&other.auth_info_bytes), ED25519_PUBKEY_TYPE_URL);
    }
}
