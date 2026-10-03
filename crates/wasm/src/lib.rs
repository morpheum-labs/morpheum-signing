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

// ==================== TYPESCRIPT SHAPES FOR `JsValue` RETURNS ====================

/// Hand-written TypeScript for the shapes `wasm-bindgen` cannot infer.
///
/// # The one rule
///
/// **Never declare a symbol `wasm_bindgen` already generates.** Everything with
/// a Rust signature — every `#[wasm_bindgen]` function, every exported class —
/// is declared by the generated `.d.ts`, and that generated declaration is the
/// single source of truth. This section is only for the *interior* of values
/// typed `JsValue`, which `wasm-bindgen` can only render as `any`. Each
/// interface below is bound to its producing signature by an
/// `unchecked_return_type` / `unchecked_param_type` attribute in `bindings.rs`,
/// so a shape cannot drift out of use unnoticed.
///
/// # Why the rule is absolute
///
/// A hand-written copy of a generated declaration drifts from the Rust it
/// describes, and TypeScript does not report that as a conflict — it *merges*
/// it, and the two merge modes are each a distinct failure:
///
/// - **Functions become an overload set.** A stale declaration with fewer
///   parameters keeps type-checking calls the real function rejects, so a
///   required parameter (such as `nonce`) stops being enforced by the
///   compiler.
/// - **Classes collide outright** (`TS2300: Duplicate identifier`), which makes
///   the package's own `.d.ts` invalid TypeScript and pushes every consumer to
///   set `skipLibCheck: true` — which in turn hides the overload problem above.
///
/// A generated declaration cannot go stale; a hand-written copy of one always
/// can.
#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Morpheum Signing SDK — shapes for the values typed `any` by wasm-bindgen.
 *
 * Signatures live in the generated declarations, not here. See the Rust doc
 * comment on `TS_TYPES` for why that separation is load-bearing.
 */

/**
 * Everything `buildSignDocBytes` binds into a signing preimage.
 *
 * One named object rather than ten positional parameters, so a missing field
 * is a TypeScript error rather than a silently defaulted trailing argument.
 *
 * `nonce` is required: the signature must cover the exact nonce the
 * transaction carries. `memo`, `accountNumber` and `genesisHash` are optional
 * because absent and default are genuinely the same statement for those
 * three — an absent `genesisHash` produces a preimage that binds no chain
 * instance (see `genesisHash` below).
 * `gasLimit` is optional for a different reason: an absent (or `null`) one
 * declares the SDK's default gas limit, itself a valid declaration, while a
 * supplied number is never replaced.
 */
export interface SignDocRequest {
    /** Protobuf type URL (e.g. "/bucket.v1.MsgCreateBucketRequest"). */
    typeUrl: string;
    /** Pre-encoded protobuf message bytes. */
    msgBytes: Uint8Array;
    /** Hex-encoded signer key — 20-byte EVM address or 32-byte Ed25519 key. */
    signerAddress: string;
    /** `ChainType` enum value (1 = Ethereum, 2 = Solana, 3 = Bitcoin). */
    chainType: number;
    /** `SignMode` enum value. */
    signMode: number;
    /** Chain identifier (e.g. "morm-dev-1"). */
    chainId: string;
    /** Optional transaction memo. */
    memo?: string;
    /** Optional account number; defaults to 0. */
    accountNumber?: bigint;
    /**
     * The target chain's 32-byte genesis hash, which stops a signature valid
     * on one chain being replayed onto another sharing its `chainId`. Omit it
     * only if you accept a signature that binds no chain instance; take it
     * from operator configuration, never from the node you submit to.
     */
    genesisHash?: Uint8Array;
    /**
     * Proto-encoded `Nonce` to bind. **Required.** Whatever is passed here is
     * returned in {@link SignDocBytes.nonce} and is the only value that will
     * verify, because it is the one the signature covers.
     */
    nonce: Uint8Array;
    /**
     * Gas limit to declare in `AuthInfo.gas_limit`, which the signature covers.
     * Absent (or `null`) declares the SDK default, sized for native-module
     * messages with a fixed cost. A contract (VM) message, or one whose cost
     * grows with the work it does, can need more and must then declare it.
     * Every declared unit is reserved against the block's gas budget whether
     * used or not, so declare what the transaction needs. Must be at least 1
     * and at most the chain's per-transaction gas budget: anything else is an
     * error, never replaced by the default.
     */
    gasLimit?: bigint;
}

/**
 * The canonical SignDoc bundle returned by `buildSignDocBytes`.
 *
 * `nonce` is the encoding the preimage actually bound, and it is on this
 * interface for the same reason the parameter is required: the caller must
 * stamp *this* value onto `Tx.nonce` — it is the only nonce the signature
 * covers.
 */
export interface SignDocBytes {
    /** SignDoc proto-encoded bytes — the signing preimage. */
    signDocBytes: Uint8Array;
    /** `hex(SHA-256(signDocBytes))`. */
    signDocHash: string;
    /** TxBody proto-encoded bytes. */
    bodyBytes: Uint8Array;
    /** AuthInfo proto-encoded bytes. */
    authInfoBytes: Uint8Array;
    /** The `Nonce` this preimage bound — stamp exactly this onto `Tx.nonce`. */
    nonce: Uint8Array;
}

/**
 * Fully signed transaction, ready for broadcast.
 *
 * Field-for-field what `TxBuilderWasm.sign()` assembles; see its Rust body.
 */
export interface SignedTx {
    /** Raw signed bytes (TxRaw encoded) — pass directly to the broadcast endpoint. */
    raw_bytes: Uint8Array;
    /** Full `tx.v1.Tx` protobuf bytes, for inspection and debugging. */
    tx_bytes: Uint8Array;
    /** `hex(SHA-256(raw_bytes))`. */
    txhash: string;
    /** `TxRaw` protobuf bytes, present only when the signer produced one. */
    tx_raw_bytes?: Uint8Array;
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
