# Native sign

A caller builds a `NativeSigner` and signs one `Tx` with `native(signer)`. The example prints the transaction hash and the length of the raw signed bytes.

## Sub-features

- `native-banner` prints `Morpheum Native Signing Example`.
- `native-sign-ok` prints `Transaction signed successfully!`.
- `native-txhash` prints `TxHash` as 64 lowercase hex characters, the SHA-256 of the raw signed bytes.
- `native-raw-bytes` prints `Raw bytes len` as a positive integer.

## How to get to it (user POV)

- In Rust, with package `morpheum-signing-native` and its default features, call `NativeSigner::from_mnemonic` on the public BIP-39 abandon phrase and an empty passphrase. The example in `crates/native/examples/native.rs` does that, and falls back to `NativeSigner::from_seed(&[42u8; 32])` only when the `bip39` feature is off.
- Call `native(signer).chain_id("morpheum-test-1").with_genesis_hash([0x5A_u8; 32])`, set memo `Test market creation from native signer`, add an `Any` whose `type_url` is `type.googleapis.com/market.v1.MsgCreateMarketRequest`, and `sign().await`.
- Read stdout. The lines a caller checks are the success line, `TxHash`, and `Raw bytes len`.

## Driving it with verify-morpheum-signing

Preconditions:

- `verify-morpheum-signing.sh doctor` prints `doctor=OK` for this `VERIFY_RUN_ID`.
- `verify-morpheum-signing.sh launch` prints `launch=OK` and `launch/result.txt` is `PASS`.
- `launch/binaries.txt` lists example `native`.

1. Run `.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh drive-native-sign`.
2. Require exit code 0.
3. Require stdout to contain the exact line `Transaction signed successfully!`.
4. Require a stdout line `  TxHash          : ` plus 64 lowercase hex characters. Reject 64 zeros.
5. Require a stdout line `  Raw bytes len   : ` plus an integer of at least 1, then ` bytes`.
6. Require `evidence/$VERIFY_RUN_ID/native-sign/result.txt` and `evidence/$VERIFY_RUN_ID/result.txt` to be the single line `PASS`.
7. Require `native-sign/binary-sha256.txt` to match the `native` hash in `launch/binaries.txt`.

## Gotchas

- `cargo test` is not this drive. The proof is the example binary's stdout and exit code.
- Default features are required. Launch does not pass `--no-default-features`. Without `bip39` and `tokio` the example does not build as written.
- The example's seed `[42u8; 32]`, genesis hash `[0x5A_u8; 32]`, and abandon mnemonic are public constants already in the file. Do not replace them with a live key. Do not record a private key into evidence.
- The helper checks the shape of `TxHash` and `Raw bytes len`. It does not pin the hash bytes. A product change that alters the preimage can still pass this drive when the example still signs and prints those lines.
- The `TxHash` spacing is ten spaces between the word and the colon, copied from `examples/native.rs`. The claim-verification example uses a shorter gap. A check written for one example fails on the other.
- A missing sibling fails doctor. Do not satisfy `path = "../morpheum-primitives"` or the `../cryptogram/crates/...` entries with a stub.
- Cleanup deletes the binary. Launch again before another drive of the same run id.
