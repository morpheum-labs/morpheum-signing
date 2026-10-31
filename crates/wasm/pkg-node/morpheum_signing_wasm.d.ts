/* tslint:disable */
/* eslint-disable */

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



/**
 * WASM-friendly transaction builder for browser frontends (React, Vue, Svelte, Next.js, etc.).
 *
 * **Completely generic** — messages are added via `add_message(type_url, value)`.
 * Use the async factory methods (`newMetamask()`, `newPhantom()`, `newTaproot()`)
 * which connect to the injected wallet and cache the public key before returning.
 */
export class TxBuilderWasm {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Sets the account number.
     */
    accountNumber(account_number: bigint): TxBuilderWasm;
    /**
     * **Generic message adder** — the only way to add messages.
     * Pass the protobuf type URL and encoded bytes as a `Uint8Array`.
     */
    addMessage(type_url: string, value: Uint8Array): TxBuilderWasm;
    /**
     * Sets the chain ID.
     */
    chainId(chain_id: string): TxBuilderWasm;
    /**
     * Declares the transaction's gas limit, replacing the default
     * (`DEFAULT_GAS_LIMIT`, sized for native-module messages with a fixed
     * cost).
     *
     * The limit is signed and reserved in full against the block's gas
     * budget, so declare what the transaction needs. `0` or a value above
     * the per-transaction budget is refused here, before anything is signed.
     */
    gasLimit(gas_limit: bigint): TxBuilderWasm;
    /**
     * Sets an optional memo.
     */
    memo(memo: string): TxBuilderWasm;
    /**
     * Creates a builder backed by **MetaMask** (or any EVM injected wallet).
     *
     * Connects to `window.ethereum`, requests account access, and caches the
     * EVM address. Returns an error if MetaMask is not available or the user
     * rejects the connection.
     */
    static newMetamask(): Promise<TxBuilderWasm>;
    /**
     * Creates a builder backed by **Phantom** (or any Solana injected wallet).
     *
     * Connects to `window.phantom.solana`, requests wallet access, and caches
     * the ed25519 public key (32 bytes).
     */
    static newPhantom(): Promise<TxBuilderWasm>;
    /**
     * Creates a builder backed by **Unisat / Leather / Xverse** (Bitcoin Taproot).
     *
     * Connects to `window.unisat`, requests account access, and caches
     * the BIP-340 x-only Schnorr public key (32 bytes).
     */
    static newTaproot(): Promise<TxBuilderWasm>;
    /**
     * Final signing call — returns a Promise that resolves to `SignedTx`.
     *
     * This builds the transaction, embeds the claim (if present), signs it
     * with the connected wallet, and returns the fully signed transaction.
     *
     * The result is the `SignedTx` shape declared in `lib.rs`.
     */
    sign(): Promise<SignedTx>;
    /**
     * Sets timeout in seconds since epoch.
     */
    timeoutSeconds(seconds: bigint): TxBuilderWasm;
    /**
     * Attaches a `TradingKeyClaim` for agent delegation.
     *
     * The claim object is the `TradingKeyClaimInput` shape declared in
     * `lib.rs`; `TradingKeyClaimJs` below is the Rust mirror it deserialises
     * into, and the two must stay field-for-field identical.
     */
    withClaim(claim_js: TradingKeyClaimInput): TxBuilderWasm;
    /**
     * Binds the target chain's genesis hash into the signing preimage, so a
     * signature valid on this chain cannot be replayed onto another chain
     * sharing the same `chainId`.
     *
     * Required: `sign()` refuses to build without it. Take the value from
     * operator configuration, never from the node the transaction is
     * submitted to.
     */
    withGenesisHash(genesis_hash: Uint8Array): TxBuilderWasm;
}

/**
 * Standalone wasm_bindgen wrapper for building a `TradingKeyClaim` from JS.
 *
 * Usage from TypeScript:
 * ```typescript
 * const claim = new VcClaimBuilderWasm()
 *     .issuer(issuerBytes)
 *     .subject(subjectBytes)
 *     .permissions(0x01n)
 *     .maxDailyUsd(10000n)
 *     .expiry(BigInt(Math.floor(Date.now() / 1000) + 86400))
 *     .nonceSubRange(100, 200)
 *     .signature(sigBytes, "ed25519")
 *     .build(BigInt(Math.floor(Date.now() / 1000)));
 * ```
 */
export class VcClaimBuilder {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Builds and validates the claim. `current_timestamp` is Unix seconds.
     */
    build(current_timestamp: bigint): TradingKeyClaimBuilt;
    /**
     * Sets the expiry timestamp (Unix seconds).
     */
    expiry(timestamp: bigint): VcClaimBuilder;
    /**
     * Sets the issuer AccountId (32 bytes).
     */
    issuer(bytes: Uint8Array): VcClaimBuilder;
    /**
     * Sets the daily USD spending limit.
     */
    maxDailyUsd(amount: bigint): VcClaimBuilder;
    /**
     * Creates a new empty builder.
     */
    constructor();
    /**
     * Sets the nonce sub-range [start, end).
     */
    nonceSubRange(start: number, end: number): VcClaimBuilder;
    /**
     * Sets the permission bitflags.
     */
    permissions(perms: bigint): VcClaimBuilder;
    /**
     * Sets the issuer's signature (64 bytes) and signature type.
     */
    signature(sig_bytes: Uint8Array, sig_type: string): VcClaimBuilder;
    /**
     * Sets the subject AccountId (32 bytes).
     */
    subject(bytes: Uint8Array): VcClaimBuilder;
}

/**
 * Constructs canonical SignDoc bytes without requiring a connected wallet.
 *
 * The signer-less counterpart to `TxBuilder::sign`. Takes one named request
 * object rather than ten positional parameters — a missing field is a
 * TypeScript error rather than a silently defaulted trailing argument.
 *
 * This function is only the JS boundary: it decodes the request, delegates the
 * actual assembly to
 * [`morpheum_signing_core::preimage::build_sign_doc`], and marshals the result
 * back. The assembly lives in `core` because this crate is
 * `#![cfg(target_arch = "wasm32")]` and therefore invisible to every host test
 * runner — preimage logic is the last thing that should be untestable. It is
 * pinned there by a golden vector captured from this binding.
 *
 * Returns the `SignDocBytes` shape declared in `lib.rs` and named by this
 * signature's `unchecked_return_type`.
 */
export function buildSignDocBytes(request: SignDocRequest): SignDocBytes;

/**
 * Installs a panic hook for better browser console error messages.
 */
export function setPanicHook(): void;

/**
 * Returns the SDK version string.
 */
export function version(): string;
