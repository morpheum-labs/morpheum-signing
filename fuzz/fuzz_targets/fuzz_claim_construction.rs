//! Fuzz target: TradingKeyClaim construction from arbitrary inputs.
//!
//! Verifies that VcClaimBuilder never panics on arbitrary inputs and that
//! valid claims always pass validation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use morpheum_signing_core::{
    claim::VcClaimBuilder,
    types::{AccountId, Signature},
};

#[derive(arbitrary::Arbitrary, Debug)]
struct FuzzClaimInput {
    issuer: [u8; 32],
    subject: [u8; 32],
    permissions: u64,
    max_daily_usd: u64,
    expiry: u64,
    nonce_start: u32,
    nonce_end: u32,
    signature: [u8; 64],
    current_time: u64,
    has_issuer: bool,
    has_subject: bool,
    has_expiry: bool,
    has_signature: bool,
}

fuzz_target!(|input: FuzzClaimInput| {
    let mut builder = VcClaimBuilder::new();

    if input.has_issuer {
        builder = builder.issuer(AccountId(input.issuer));
    }
    if input.has_subject {
        builder = builder.subject(AccountId(input.subject));
    }

    builder = builder
        .permissions(input.permissions)
        .max_daily_usd(input.max_daily_usd)
        .nonce_sub_range(input.nonce_start, input.nonce_end);

    if input.has_expiry {
        builder = builder.expiry(input.expiry);
    }
    if input.has_signature {
        builder = builder.signature(Signature::Ed25519(input.signature));
    }

    // Build should never panic — only return Ok or Err
    match builder.build(input.current_time) {
        Ok(claim) => {
            // Valid claims should always encode without panic
            let _ = claim.encode_to_vec();
            let _ = claim.to_proto_any();
            let _ = claim.claim_digest();
            let _ = claim.sub_range_size();

            // validate at original time should succeed (since build validates)
            assert!(claim.validate(input.current_time).is_ok());
        }
        Err(_) => {
            // Expected for invalid inputs — no panic is the requirement
        }
    }
});
