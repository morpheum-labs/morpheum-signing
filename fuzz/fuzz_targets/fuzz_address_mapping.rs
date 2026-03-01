//! Fuzz target: address mapping from arbitrary inputs.
//!
//! Verifies that `DefaultAddressMapper` never panics on any address variant,
//! including edge cases like empty strings, maximum-length strings, and
//! arbitrary byte arrays.

#![no_main]

use libfuzzer_sys::fuzz_target;
use morpheum_signing_core::{
    mapper::{AddressMapper, DefaultAddressMapper},
    types::Address,
};

#[derive(arbitrary::Arbitrary, Debug)]
enum FuzzAddress {
    Native(String),
    Evm([u8; 20]),
    Solana([u8; 32]),
    Bitcoin(String),
    Agent(String),
}

fuzz_target!(|addr: FuzzAddress| {
    let mapper = DefaultAddressMapper;

    let address = match addr {
        FuzzAddress::Native(s) => Address::Native(s),
        FuzzAddress::Evm(b) => Address::Evm(b),
        FuzzAddress::Solana(b) => Address::Solana(b),
        FuzzAddress::Bitcoin(s) => Address::Bitcoin(s),
        FuzzAddress::Agent(s) => Address::Agent(s),
    };

    // Should never panic, always produce a valid AccountId
    let result = mapper.to_account_id(&address);
    assert!(result.is_ok());

    // to_account_id via Address should match mapper
    let direct = address.to_account_id();
    assert_eq!(result.unwrap(), direct);
});
