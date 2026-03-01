# Security Policy — Morpheum Signing SDK

## Security Checklist

This checklist documents the security properties of the Morpheum Signing SDK
and should be reviewed before every release.

### Secret Material Handling

- [x] All signers implement `ZeroizeOnDrop` — secret key material is zeroized
      when the signer is dropped.
- [x] `ed25519-dalek::SigningKey` handles its own zeroization on `Drop`.
- [x] `k256::ecdsa::SigningKey` handles its own zeroization on `Drop`.
- [x] `BitcoinSigner` explicitly zeroizes cached public key bytes in `Drop`.
- [x] BIP-39 intermediate seed material is zeroized after key derivation
      (`NativeSigner::from_mnemonic`).
- [x] No secret material appears in `Debug`, `Display`, log output, or panic messages.
- [x] `Signature` enum implements `Zeroize` and `ZeroizeOnDrop`.

### Constant-Time Signing

- [x] **ed25519-dalek** (NativeSigner, AgentSigner, SolanaSigner): constant-time
      scalar multiplication and deterministic nonce (RFC 8032).
- [x] **k256** (EvmSigner): constant-time ECDSA via `crypto-bigint`.
- [x] **libsecp256k1** (BitcoinSigner): constant-time BIP-340 Schnorr with
      `sign_schnorr_no_aux_rand` (deterministic, no auxiliary randomness).
- [x] No secret-dependent branching in any signing path.

### TradingKeyClaim Security

- [x] Claims are validated for structural correctness before embedding: expiry,
      nonce sub-range validity, non-zero signature.
- [x] `claim_digest()` uses prost deterministic encoding + SHA-256 for
      the canonical digest that issuers sign.
- [x] The `signature` field is excluded from the digest (it _is_ the signature
      over the digest).
- [x] `verify()` checks issuer `AccountId` derivation from the provided public key.
- [x] Full cryptographic signature verification is deferred to the chain-side
      VC hot-path (curve-agnostic, authoritative). The SDK provides the digest
      via `claim_digest()` for offline verification.
- [x] Claims are embedded in `SignerInfo.signing_options` as `prost_types::Any`
      for forward-compatible protobuf encoding.

### Address Mapping

- [x] `DefaultAddressMapper` uses SHA-256 hash for all address types.
- [x] Address mapping is deterministic and reproducible.
- [x] Empty addresses produce a valid (non-zero) `AccountId` — this is by design
      (SHA-256 of empty input is not zero).

### WASM Security

- [x] The WASM crate does not expose secret key material to JavaScript.
- [x] `RefCell` is used for interior mutability in WASM adapters (safe in
      single-threaded WASM).
- [x] `unsafe impl Send/Sync` for `WasmSigner` is sound because WASM is
      single-threaded by specification.
- [x] `getrandom` uses the `js` feature for secure randomness in WASM.

### Build & Feature Flags

- [x] New capabilities are gated behind feature flags (`dynamic-signer-info`,
      `claim-verification`, `bip39`).
- [x] The `full` feature enables all features for production use.
- [x] `no_std` compatibility is maintained in the core crate.
- [x] `tonic`/`tokio`/`mio` are excluded from WASM builds via feature gating
      in `morm-proto`.

### Testing

- [x] 98+ integration tests covering all signer types, claim flows, error cases,
      and edge cases.
- [x] Deterministic test seeds and nonce providers for reproducibility.
- [x] Negative-path tests for all error variants.
- [x] Boundary tests for claim expiry, nonce sub-ranges, and max values.
- [x] Cross-signer tests verifying different curves produce different signatures.
- [x] BIP-39 mnemonic tests (valid, invalid, deterministic, passphrase).
- [x] Fuzz targets for seed generation, claim construction, and address mapping.

## Reporting Vulnerabilities

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT** open a public GitHub issue.
2. Email security@morpheum.xyz with a detailed description.
3. Include steps to reproduce if possible.
4. We will respond within 48 hours and coordinate disclosure.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.2.x   | ✅        |
| 0.1.x   | ⚠️ Upgrade recommended |
| < 0.1   | ❌        |
