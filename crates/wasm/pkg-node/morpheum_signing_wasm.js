/* @ts-self-types="./morpheum_signing_wasm.d.ts" */

/**
 * WASM-friendly transaction builder for browser frontends (React, Vue, Svelte, Next.js, etc.).
 *
 * **Completely generic** — messages are added via `add_message(type_url, value)`.
 * Use the async factory methods (`newMetamask()`, `newPhantom()`, `newTaproot()`)
 * which connect to the injected wallet and cache the public key before returning.
 */
class TxBuilderWasm {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(TxBuilderWasm.prototype);
        obj.__wbg_ptr = ptr;
        TxBuilderWasmFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TxBuilderWasmFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_txbuilderwasm_free(ptr, 0);
    }
    /**
     * Sets the account number.
     * @param {bigint} account_number
     * @returns {TxBuilderWasm}
     */
    accountNumber(account_number) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.txbuilderwasm_accountNumber(ptr, account_number);
        return TxBuilderWasm.__wrap(ret);
    }
    /**
     * **Generic message adder** — the only way to add messages.
     * Pass the protobuf type URL and encoded bytes as a `Uint8Array`.
     * @param {string} type_url
     * @param {Uint8Array} value
     * @returns {TxBuilderWasm}
     */
    addMessage(type_url, value) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(type_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(value, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.txbuilderwasm_addMessage(ptr, ptr0, len0, ptr1, len1);
        return TxBuilderWasm.__wrap(ret);
    }
    /**
     * Sets the chain ID.
     * @param {string} chain_id
     * @returns {TxBuilderWasm}
     */
    chainId(chain_id) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(chain_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.txbuilderwasm_chainId(ptr, ptr0, len0);
        return TxBuilderWasm.__wrap(ret);
    }
    /**
     * Declares the transaction's gas limit, replacing the default
     * (`DEFAULT_GAS_LIMIT`, sized for native-module messages with a fixed
     * cost).
     *
     * The limit is signed and reserved in full against the block's gas
     * budget, so declare what the transaction needs. `0` or a value above
     * the per-transaction budget is refused here, before anything is signed.
     * @param {bigint} gas_limit
     * @returns {TxBuilderWasm}
     */
    gasLimit(gas_limit) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.txbuilderwasm_gasLimit(ptr, gas_limit);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return TxBuilderWasm.__wrap(ret[0]);
    }
    /**
     * Sets an optional memo.
     * @param {string} memo
     * @returns {TxBuilderWasm}
     */
    memo(memo) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passStringToWasm0(memo, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.txbuilderwasm_memo(ptr, ptr0, len0);
        return TxBuilderWasm.__wrap(ret);
    }
    /**
     * Creates a builder backed by **MetaMask** (or any EVM injected wallet).
     *
     * Connects to `window.ethereum`, requests account access, and caches the
     * EVM address. Returns an error if MetaMask is not available or the user
     * rejects the connection.
     * @returns {Promise<TxBuilderWasm>}
     */
    static newMetamask() {
        const ret = wasm.txbuilderwasm_newMetamask();
        return ret;
    }
    /**
     * Creates a builder backed by **Phantom** (or any Solana injected wallet).
     *
     * Connects to `window.phantom.solana`, requests wallet access, and caches
     * the ed25519 public key (32 bytes).
     * @returns {Promise<TxBuilderWasm>}
     */
    static newPhantom() {
        const ret = wasm.txbuilderwasm_newPhantom();
        return ret;
    }
    /**
     * Creates a builder backed by **Unisat / Leather / Xverse** (Bitcoin Taproot).
     *
     * Connects to `window.unisat`, requests account access, and caches
     * the BIP-340 x-only Schnorr public key (32 bytes).
     * @returns {Promise<TxBuilderWasm>}
     */
    static newTaproot() {
        const ret = wasm.txbuilderwasm_newTaproot();
        return ret;
    }
    /**
     * Final signing call — returns a Promise that resolves to `SignedTx`.
     *
     * This builds the transaction, embeds the claim (if present), signs it
     * with the connected wallet, and returns the fully signed transaction.
     *
     * The result is the `SignedTx` shape declared in `lib.rs`.
     * @returns {SignedTx}
     */
    sign() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.txbuilderwasm_sign(ptr);
        return ret;
    }
    /**
     * Sets timeout in seconds since epoch.
     * @param {bigint} seconds
     * @returns {TxBuilderWasm}
     */
    timeoutSeconds(seconds) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.txbuilderwasm_timeoutSeconds(ptr, seconds);
        return TxBuilderWasm.__wrap(ret);
    }
    /**
     * Attaches a `TradingKeyClaim` for agent delegation.
     *
     * The claim object is the `TradingKeyClaimInput` shape declared in
     * `lib.rs`; `TradingKeyClaimJs` below is the Rust mirror it deserialises
     * into, and the two must stay field-for-field identical.
     * @param {TradingKeyClaimInput} claim_js
     * @returns {TxBuilderWasm}
     */
    withClaim(claim_js) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.txbuilderwasm_withClaim(ptr, claim_js);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return TxBuilderWasm.__wrap(ret[0]);
    }
    /**
     * Binds the target chain's genesis hash into the signing preimage, so a
     * signature valid on this chain cannot be replayed onto another chain
     * sharing the same `chainId`.
     *
     * Required: `sign()` refuses to build without it. Take the value from
     * operator configuration, never from the node the transaction is
     * submitted to.
     * @param {Uint8Array} genesis_hash
     * @returns {TxBuilderWasm}
     */
    withGenesisHash(genesis_hash) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passArray8ToWasm0(genesis_hash, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.txbuilderwasm_withGenesisHash(ptr, ptr0, len0);
        return TxBuilderWasm.__wrap(ret);
    }
}
if (Symbol.dispose) TxBuilderWasm.prototype[Symbol.dispose] = TxBuilderWasm.prototype.free;
exports.TxBuilderWasm = TxBuilderWasm;

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
class VcClaimBuilder {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(VcClaimBuilder.prototype);
        obj.__wbg_ptr = ptr;
        VcClaimBuilderFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        VcClaimBuilderFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_vcclaimbuilder_free(ptr, 0);
    }
    /**
     * Builds and validates the claim. `current_timestamp` is Unix seconds.
     * @param {bigint} current_timestamp
     * @returns {TradingKeyClaimBuilt}
     */
    build(current_timestamp) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.vcclaimbuilder_build(ptr, current_timestamp);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Sets the expiry timestamp (Unix seconds).
     * @param {bigint} timestamp
     * @returns {VcClaimBuilder}
     */
    expiry(timestamp) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.vcclaimbuilder_expiry(ptr, timestamp);
        return VcClaimBuilder.__wrap(ret);
    }
    /**
     * Sets the issuer AccountId (32 bytes).
     * @param {Uint8Array} bytes
     * @returns {VcClaimBuilder}
     */
    issuer(bytes) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.vcclaimbuilder_issuer(ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return VcClaimBuilder.__wrap(ret[0]);
    }
    /**
     * Sets the daily USD spending limit.
     * @param {bigint} amount
     * @returns {VcClaimBuilder}
     */
    maxDailyUsd(amount) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.vcclaimbuilder_maxDailyUsd(ptr, amount);
        return VcClaimBuilder.__wrap(ret);
    }
    /**
     * Creates a new empty builder.
     */
    constructor() {
        const ret = wasm.vcclaimbuilder_new();
        this.__wbg_ptr = ret >>> 0;
        VcClaimBuilderFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Sets the nonce sub-range [start, end).
     * @param {number} start
     * @param {number} end
     * @returns {VcClaimBuilder}
     */
    nonceSubRange(start, end) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.vcclaimbuilder_nonceSubRange(ptr, start, end);
        return VcClaimBuilder.__wrap(ret);
    }
    /**
     * Sets the permission bitflags.
     * @param {bigint} perms
     * @returns {VcClaimBuilder}
     */
    permissions(perms) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.vcclaimbuilder_permissions(ptr, perms);
        return VcClaimBuilder.__wrap(ret);
    }
    /**
     * Sets the issuer's signature (64 bytes) and signature type.
     * @param {Uint8Array} sig_bytes
     * @param {string} sig_type
     * @returns {VcClaimBuilder}
     */
    signature(sig_bytes, sig_type) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passArray8ToWasm0(sig_bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(sig_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.vcclaimbuilder_signature(ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return VcClaimBuilder.__wrap(ret[0]);
    }
    /**
     * Sets the subject AccountId (32 bytes).
     * @param {Uint8Array} bytes
     * @returns {VcClaimBuilder}
     */
    subject(bytes) {
        const ptr = this.__destroy_into_raw();
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.vcclaimbuilder_subject(ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return VcClaimBuilder.__wrap(ret[0]);
    }
}
if (Symbol.dispose) VcClaimBuilder.prototype[Symbol.dispose] = VcClaimBuilder.prototype.free;
exports.VcClaimBuilder = VcClaimBuilder;

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
 * @param {SignDocRequest} request
 * @returns {SignDocBytes}
 */
function buildSignDocBytes(request) {
    const ret = wasm.buildSignDocBytes(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
exports.buildSignDocBytes = buildSignDocBytes;

/**
 * Installs a panic hook for better browser console error messages.
 */
function setPanicHook() {
    wasm.setPanicHook();
}
exports.setPanicHook = setPanicHook;

/**
 * Returns the SDK version string.
 * @returns {string}
 */
function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
exports.version = version;

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_83742b46f01ce22d: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_a5a435bd7bbec835: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_bigint_get_as_i64_447a76b5c6ef7bda: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_c0f3f60bac5a78d1: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_5398f5bb970e0daa: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_41dbb8413020e076: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_e2141d4f045b7eda: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_3c846841762788c1: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_0b605fc6b167c56f: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_781bc9f159099513: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_52709e72fb9f179c: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_ee31bfad3e536463: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_5bcc3bed3c69e72b: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_34bb9d9dcfa21373: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_395e606bd0ee4427: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_6ddd609b62940d55: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_6b5b6b8576d35cb1: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_call_2d781c1f4d5c0ef8: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_call_e133b57c9155d22c: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_connect_71d9ffe09d7ce546: function(arg0) {
            const ret = arg0.connect();
            return ret;
        },
        __wbg_done_08ce71ee07e3bd17: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_from_4bdf88943703fd48: function(arg0) {
            const ret = Array.from(arg0);
            return ret;
        },
        __wbg_getPublicKey_4d61a2aa7724a76c: function(arg0) {
            const ret = arg0.getPublicKey();
            return ret;
        },
        __wbg_get_326e41e095fb2575: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_3ef1eba1850ade27: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_a8ee5c45dabc1b3b: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_329cfe50afab7352: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_6412cf3094599694: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_101e2bf31071a9f6: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_740438561a5b956d: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_23e677d2c6843922: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_33b91feb269ff46e: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isSafeInteger_ecd6a7f9c3e053cd: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_d8f549ec8fb061b1: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_length_b3416cf66a5452c8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_ea16607d7b61445b: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_5f486cdf45a04d78: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_a70fbab9066b301f: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_ab79df5bd7c26067: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_from_slice_22da9388ac046e50: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_aaaeaf29cf802876: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_2aef5a05128f03d9___convert__closures_____invoke___js_sys_a6b7100d4d5f5390___Function_fn_wasm_bindgen_2aef5a05128f03d9___JsValue_____wasm_bindgen_2aef5a05128f03d9___sys__Undefined___js_sys_a6b7100d4d5f5390___Function_fn_wasm_bindgen_2aef5a05128f03d9___JsValue_____wasm_bindgen_2aef5a05128f03d9___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = state0.b = 0;
            }
        },
        __wbg_next_11b99ee6237339e3: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_next_e01a967809d1aa68: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_of_d6376e3774c51f89: function(arg0, arg1) {
            const ret = Array.of(arg0, arg1);
            return ret;
        },
        __wbg_prototypesetcall_d62e5099504357e6: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_e87b0e732085a946: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_0c399741342fb10f: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_a082d78ce798393e: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_requestAccounts_9ef506dc340ba2bf: function(arg0) {
            const ret = arg0.requestAccounts();
            return ret;
        },
        __wbg_request_32e5db646a3c3b69: function(arg0, arg1) {
            const ret = arg0.request(arg1);
            return ret;
        },
        __wbg_resolve_ae8d83246e5bcc12: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_set_7eaa4f96924fd6b3: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_signMessage_47e99fa866dcec7c: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.signMessage(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        },
        __wbg_signMessage_c77932640d1da314: function(arg0, arg1) {
            const ret = arg0.signMessage(arg1);
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_8adb955bd33fac2f: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_ad356e0db91c7913: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_f207c857566db248: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_bb9f1ba69d61b386: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stringify_5ae93966a84901ac: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_then_098abe61755d12f6: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_9e335f6dd892bc11: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_txbuilderwasm_new: function(arg0) {
            const ret = TxBuilderWasm.__wrap(arg0);
            return ret;
        },
        __wbg_value_21fc78aab0322612: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 20, function: Function { arguments: [Externref], shim_idx: 21, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen_2aef5a05128f03d9___closure__destroy___dyn_core_ed718c3d60ebd546___ops__function__FnMut__wasm_bindgen_2aef5a05128f03d9___JsValue____Output___core_ed718c3d60ebd546___result__Result_____wasm_bindgen_2aef5a05128f03d9___JsError___, wasm_bindgen_2aef5a05128f03d9___convert__closures_____invoke___wasm_bindgen_2aef5a05128f03d9___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_2aef5a05128f03d9___JsError___true_);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./morpheum_signing_wasm_bg.js": import0,
    };
}

function wasm_bindgen_2aef5a05128f03d9___convert__closures_____invoke___wasm_bindgen_2aef5a05128f03d9___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_2aef5a05128f03d9___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_2aef5a05128f03d9___convert__closures_____invoke___wasm_bindgen_2aef5a05128f03d9___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_2aef5a05128f03d9___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_2aef5a05128f03d9___convert__closures_____invoke___js_sys_a6b7100d4d5f5390___Function_fn_wasm_bindgen_2aef5a05128f03d9___JsValue_____wasm_bindgen_2aef5a05128f03d9___sys__Undefined___js_sys_a6b7100d4d5f5390___Function_fn_wasm_bindgen_2aef5a05128f03d9___JsValue_____wasm_bindgen_2aef5a05128f03d9___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_2aef5a05128f03d9___convert__closures_____invoke___js_sys_a6b7100d4d5f5390___Function_fn_wasm_bindgen_2aef5a05128f03d9___JsValue_____wasm_bindgen_2aef5a05128f03d9___sys__Undefined___js_sys_a6b7100d4d5f5390___Function_fn_wasm_bindgen_2aef5a05128f03d9___JsValue_____wasm_bindgen_2aef5a05128f03d9___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}

const TxBuilderWasmFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_txbuilderwasm_free(ptr >>> 0, 1));
const VcClaimBuilderFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_vcclaimbuilder_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => state.dtor(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
function decodeText(ptr, len) {
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

const wasmPath = `${__dirname}/morpheum_signing_wasm_bg.wasm`;
const wasmBytes = require('fs').readFileSync(wasmPath);
const wasmModule = new WebAssembly.Module(wasmBytes);
let wasm = new WebAssembly.Instance(wasmModule, __wbg_get_imports()).exports;
wasm.__wbindgen_start();
