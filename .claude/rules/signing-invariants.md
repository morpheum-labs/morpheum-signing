# Signing invariants — crates/core and crates/wasm

Byte-level rules; a violation here changes what a signature covers.

- All preimage construction flows through the canonical builder in `crates/core`; no other
  crate (and no downstream SDK) may assemble `SignDoc` bytes independently.
- Guard order in `TxBuilder::sign()` is load-bearing: fail-closed checks run **before**
  nonce resolution so a refusal never consumes a monotonic provider nonce. The ordering is
  observable only through the counting-provider test — do not weaken it.
- Golden-vector constants: single unbroken literals, captured from the shipped package.
  Changing them is changing the signature format — that is an ecosystem event, not a
  refactor.
- `crates/wasm`: interfaces only in `typescript_custom_section`; never declare a symbol
  wasm-bindgen generates; keep `nonce` required-and-returned. Any change here requires
  `wasm-pack build crates/wasm --target nodejs --out-dir pkg-node` plus a typecheck of the
  TS consumer before the PR.
- New chain/signature types: implement `Signer` + verifier symmetrically, add fuzz coverage
  for address mapping, and keep secret material `ZeroizeOnDrop` end to end (no `Debug` on
  secret-bearing types).
