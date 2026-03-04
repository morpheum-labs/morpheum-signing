//! WASM entrypoint for the Morpheum Signing SDK.
//!
//! Provides clean, TypeScript-friendly bindings for browser frontends.
//! Supports injected wallets (MetaMask, Phantom, Taproot) via complete
//! [`Signer`](morpheum_signing_core::signer::Signer) adapter implementations
//! with interior mutability for cached wallet state.
//!
//! Build with: `wasm-pack build crates/wasm --target web --release`

#![cfg(target_arch = "wasm32")]
#![allow(non_snake_case)]

use wasm_bindgen::prelude::*;

use morpheum_signing_core as core;

pub(crate) mod adapters;
mod bindings;

// ==================== PANIC HOOK FOR BETTER DEBUGGING ====================

/// Installs a panic hook for better browser console error messages.
#[wasm_bindgen(js_name = "setPanicHook")]
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// ==================== VERSION ====================

/// Returns the SDK version string.
#[wasm_bindgen]
pub fn version() -> String {
    core::VERSION.to_string()
}

// ==================== RICH TYPESCRIPT DEFINITIONS ====================

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Morpheum Signing SDK — TypeScript Definitions
 *
 * Complete type definitions for the WASM signing library.
 * Includes transaction builder, claim support, and all wallet adapters.
 */

/** Fully signed transaction, ready for broadcast. */
export interface SignedTx {
    /** Full decoded tx.v1.Tx */
    tx: any;
    /** Raw signed bytes (TxRaw encoded) — pass directly to broadcast endpoint */
    raw_bytes: Uint8Array;
    /** Optional decoded TxRaw (for verification/debugging) */
    tx_raw?: any;
}

/** Signing options for the builder. */
export interface SigningOptions {
    deadline_seconds?: number;
    memo?: string;
    include_timestamp: boolean;
}

/**
 * TradingKeyClaim for agent delegation.
 *
 * Enables secondary keys (TradingKeys) to sign with isolated nonce sub-ranges
 * while respecting owner-defined limits.
 */
export interface TradingKeyClaimInput {
    /** Issuer AccountId (32 bytes) */
    issuer: Uint8Array;
    /** Subject AccountId (32 bytes) */
    subject: Uint8Array;
    /** Permission bitflags (e.g., TRADE=0x01, EVALUATE=0x02) */
    permissions: number;
    /** Daily USD spending limit */
    max_daily_usd: number;
    /** Expiry timestamp (Unix seconds) */
    expiry_timestamp: number;
    /** Nonce sub-range start (inclusive) */
    nonce_sub_range_start: number;
    /** Nonce sub-range end (exclusive) */
    nonce_sub_range_end: number;
    /** Issuer's signature over the claim (64 bytes) */
    signature: Uint8Array;
    /** Signature algorithm: "ed25519" | "secp256k1" | "schnorr" */
    signature_type: "ed25519" | "secp256k1" | "schnorr";
}

/** Built TradingKeyClaim with proto-encoded Any for direct embedding. */
export interface TradingKeyClaimBuilt extends TradingKeyClaimInput {
    /** Protobuf Any type_url for embedding in SignerInfo */
    proto_any_type_url: string;
    /** Protobuf Any encoded value */
    proto_any_value: Uint8Array;
}

/**
 * Main transaction builder for browser use.
 *
 * Completely generic — messages added via addMessage(type_url, value).
 * Factory methods are **async** because they connect to the injected wallet.
 *
 * @example
 * ```typescript
 * const builder = await TxBuilderWasm.newMetamask();
 * const signedTx = await builder
 *     .chainId("morpheum-1")
 *     .memo("Hello from MetaMask!")
 *     .addMessage("type.googleapis.com/market.v1.MsgCreateOrder", encodedBytes)
 *     .sign();
 * ```
 */
export class TxBuilderWasm {
    private constructor();

    /** MetaMask / Rabby / Ledger (EVM) — connects to window.ethereum */
    static newMetamask(): Promise<TxBuilderWasm>;

    /** Phantom / Solflare / Backpack (Solana) — connects to window.phantom.solana */
    static newPhantom(): Promise<TxBuilderWasm>;

    /** Unisat / Leather / Xverse (Bitcoin Taproot) — connects to window.unisat */
    static newTaproot(): Promise<TxBuilderWasm>;

    /** Sets the chain ID (e.g., "morpheum-1", "morpheum-test-1") */
    chainId(chain_id: string): TxBuilderWasm;

    /** Sets an optional memo */
    memo(memo: string): TxBuilderWasm;

    /** Sets the account number */
    accountNumber(account_number: number): TxBuilderWasm;

    /** Sets timeout in seconds since epoch */
    timeoutSeconds(seconds: number): TxBuilderWasm;

    /**
     * Add any protobuf message (completely generic).
     * @param type_url Full protobuf type URL (e.g., "type.googleapis.com/market.v1.MsgCreateOrder")
     * @param value Protobuf-encoded message bytes
     */
    addMessage(type_url: string, value: Uint8Array): TxBuilderWasm;

    /**
     * Attaches a TradingKeyClaim for agent delegation.
     * The claim is embedded in SignerInfo.signing_options and covered by the signature.
     */
    withClaim(claim: TradingKeyClaimInput): TxBuilderWasm;

    /** Signs the transaction and returns the fully signed result. */
    sign(): Promise<SignedTx>;
}

/**
 * Fluent builder for creating TradingKeyClaims from TypeScript.
 *
 * @example
 * ```typescript
 * const claim = new VcClaimBuilder()
 *     .issuer(issuerBytes)
 *     .subject(subjectBytes)
 *     .permissions(0x01)
 *     .maxDailyUsd(10000)
 *     .expiry(Math.floor(Date.now() / 1000) + 86400)
 *     .nonceSubRange(100, 200)
 *     .signature(sigBytes, "ed25519")
 *     .build(Math.floor(Date.now() / 1000));
 * ```
 */
export class VcClaimBuilder {
    constructor();
    issuer(bytes: Uint8Array): VcClaimBuilder;
    subject(bytes: Uint8Array): VcClaimBuilder;
    permissions(perms: number): VcClaimBuilder;
    maxDailyUsd(amount: number): VcClaimBuilder;
    expiry(timestamp: number): VcClaimBuilder;
    nonceSubRange(start: number, end: number): VcClaimBuilder;
    signature(sig_bytes: Uint8Array, sig_type: "ed25519" | "secp256k1" | "schnorr"): VcClaimBuilder;
    build(current_timestamp: number): TradingKeyClaimBuilt;
}
"#;

// ==================== PRELUDE ====================

/// Most commonly used items for WASM consumers.
///
/// Re-exports the core prelude (types, traits, proto definitions) for any
/// Rust code that may conditionally compile against the WASM crate.
pub mod prelude {
    pub use morpheum_signing_core::prelude::*;
}

// ==================== CONVENIENCE LOGGING ====================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
