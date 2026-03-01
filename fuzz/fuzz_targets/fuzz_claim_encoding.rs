//! Fuzz target: TradingKeyClaim encoding determinism.
//!
//! Verifies that encoding the same claim twice always produces identical bytes,
//! and that the digest is stable across invocations.

#![no_main]

use libfuzzer_sys::fuzz_target;
use morpheum_signing_core::{
    claim::TradingKeyClaim,
    types::{AccountId, Signature},
};

#[derive(arbitrary::Arbitrary, Debug)]
struct FuzzClaim {
    issuer: [u8; 32],
    subject: [u8; 32],
    permissions: u64,
    max_daily_usd: u64,
    expiry_timestamp: u64,
    nonce_sub_range_start: u32,
    nonce_sub_range_end: u32,
    signature: [u8; 64],
}

fuzz_target!(|input: FuzzClaim| {
    let claim = TradingKeyClaim {
        issuer: AccountId(input.issuer),
        subject: AccountId(input.subject),
        permissions: input.permissions,
        max_daily_usd: input.max_daily_usd,
        expiry_timestamp: input.expiry_timestamp,
        nonce_sub_range_start: input.nonce_sub_range_start,
        nonce_sub_range_end: input.nonce_sub_range_end,
        signature: Signature::Ed25519(input.signature),
    };

    // Encoding must be deterministic
    let bytes1 = claim.encode_to_vec();
    let bytes2 = claim.encode_to_vec();
    assert_eq!(bytes1, bytes2, "encode_to_vec must be deterministic");

    // Digest must be deterministic
    let digest1 = claim.claim_digest();
    let digest2 = claim.claim_digest();
    assert_eq!(digest1, digest2, "claim_digest must be deterministic");

    // Proto Any encoding must be deterministic
    let any1 = claim.to_proto_any();
    let any2 = claim.to_proto_any();
    assert_eq!(any1.type_url, any2.type_url);
    assert_eq!(any1.value, any2.value);

    // sub_range_size must not panic
    let _ = claim.sub_range_size();
});
