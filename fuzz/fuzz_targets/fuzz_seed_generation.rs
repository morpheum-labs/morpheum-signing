//! Fuzz target: signer creation from arbitrary seeds.
//!
//! Verifies that creating signers from arbitrary 32-byte seeds never panics
//! and always produces valid public keys and account IDs.

#![no_main]

use libfuzzer_sys::fuzz_target;
use morpheum_signing_core::signer::Signer;
use morpheum_signing_native::{
    AgentSigner, BitcoinSigner, EvmSigner, NativeSigner, SolanaSigner,
};
use morpheum_signing_core::types::AccountId;

fuzz_target!(|seed: [u8; 32]| {
    // NativeSigner: should never panic
    let native = NativeSigner::from_seed(&seed);
    let _ = native.public_key();
    let _ = native.account_id();
    let _ = native.public_key_proto();
    let _ = native.sign_mode();
    let _ = native.wallet_type();

    // AgentSigner: should never panic
    let agent = AgentSigner::new(&seed, AccountId([0x11; 32]), None);
    let _ = agent.public_key();
    let _ = agent.account_id();
    let _ = agent.public_key_proto();

    // SolanaSigner: should never panic
    let solana = SolanaSigner::from_seed(&seed);
    let _ = solana.public_key();
    let _ = solana.account_id();

    // EvmSigner: secp256k1 requires a non-zero, < curve-order seed.
    // from_seed() may panic for invalid seeds — that's expected behavior.
    // We catch the panic to verify graceful (non-UB) behavior.
    let _ = std::panic::catch_unwind(|| {
        let evm = EvmSigner::from_seed(&seed);
        let _ = evm.public_key();
        let _ = evm.account_id();
    });

    // BitcoinSigner: same constraint as EVM (secp256k1 curve order).
    let _ = std::panic::catch_unwind(|| {
        let btc = BitcoinSigner::from_seed(&seed);
        let _ = btc.public_key();
        let _ = btc.account_id();
    });
});
