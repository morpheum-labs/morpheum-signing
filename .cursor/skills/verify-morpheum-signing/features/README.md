# morpheum-signing verification map

This directory is the maintained source for verifying `morpheum-signing`. The programs a caller runs are the examples in `crates/native/examples/`. There is no UI, CLI server, or long-lived process. Read this index, then use the matching feature file.

## Baseline preconditions

- The checkout is this repo. `crates/native/Cargo.toml` names package `morpheum-signing-native`.
- Sibling directories `cryptogram`, `morpheum-primitives`, and `morpheum-proto` sit beside this repo, each with a `Cargo.toml`. `morpheum-proto` is required because primitives depends on it. Do not clone a missing sibling from the helper, and do not add a stub crate.
- Set `VERIFY_RUN_ID` to a token of letters, digits, `.`, `_`, or `-`, at most 81 characters, starting with a letter or digit.
- Run `verify-morpheum-signing.sh doctor` and require `doctor=OK`.
- Run `verify-morpheum-signing.sh launch` and require `launch/result.txt` to be `PASS` and `launch=OK` on stdout.
- The helper selects rustc 1.98.1, the pin in `.github/workflows/ci.yml`. Launch installs apt package `protobuf-compiler` when `protoc` is missing.
- Drive only the run id you just launched. Two run ids can build side by side. One run id has one scratch directory.

## Driving conventions

- Start every recipe from a passing doctor and a launch of the same `VERIFY_RUN_ID`.
- Invoke `.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh`. The script changes to the repo root for the build.
- One drive command runs one example binary from `/tmp/verify-morpheum-signing-$VERIFY_RUN_ID/target/debug/examples/`.
- Launch builds with default features. It does not pass `--locked` or `--no-default-features`. The `full` feature set is what turns on `bip39` and `tokio`. Launch restores `Cargo.lock` after resolution.
- `result.txt` is the single line `PASS` only when every pass condition in the feature file holds.
- Fmt, clippy, `cargo test`, and the wasm32 clippy leg match CI and are not features in this map.
- Cleanup removes `/tmp/verify-morpheum-signing-$VERIFY_RUN_ID` and does not remove `evidence/`.

## Proof and skip reporting

- Capture the binary path, stdout, stderr, exit code, and sha256 under the run's evidence directory.
- `evidence/$VERIFY_RUN_ID/<feature>/result.txt` is the line `PASS` or `FAIL`. The helper also writes that line to `evidence/$VERIFY_RUN_ID/result.txt` for the feature just driven.
- Record the feature id and the example name in `<feature>/command.txt`.
- A missing sibling is `doctor=NOT_READY` with the missing path. Do not report a different example as proof of this one.
- `EvmSigner` is documented in the README and implemented in `crates/native/src/signers/evm.rs`. There is no `examples/evm.rs`. It is not a feature in this map. Do not drive it with `cargo test`.

## Feature entry contract

Each feature file starts with an H1 and one paragraph, then exactly four H2 sections in this order: `Sub-features`, `How to get to it (user POV)`, `Driving it with verify-morpheum-signing`, `Gotchas`.

## Features

- [Native sign](./native-sign.md) runs `examples/native.rs`, signs with the mnemonic-backed `NativeSigner`, and prints `Transaction signed successfully!`, a 64-hex `TxHash`, and `Raw bytes len`. This is the seeded drive.
- [Mnemonic native account](./mnemonic-native.md) runs the same `native` binary and requires the BIP-39 account line from `NativeSigner::from_mnemonic`.
- [Agent claim](./agent-claim.md) runs `examples/agent.rs` and signs with `AgentSigner` plus a `TradingKeyClaim`.
- [Agent claim verification](./agent-claim-verify.md) runs `examples/agent_with_claim_verification.rs`, requires `Claim verification: PASSED`, and requires the claim embedded in `SignerInfo.signing_options`.
