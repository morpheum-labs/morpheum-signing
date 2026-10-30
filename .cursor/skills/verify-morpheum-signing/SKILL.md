---
name: verify-morpheum-signing
description: "Drive the morpheum-signing native examples (ed25519 sign, BIP-39 mnemonic, agent TradingKey claim) and record pass/fail evidence. Use when proving this checkout signs a transaction, or when a change to the native examples needs an end-to-end check."
---

# Verify morpheum-signing

`morpheum-signing` is a Rust library. The programs a caller runs are the examples under `crates/native/examples/`. There is no server and no UI. Launch checks sibling checkouts, installs the CI toolchain, and builds those examples. Each drive runs one built example binary and reads its stdout.

The helper is `.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh`. It finds the repo from its own path. Run it from any working directory.

`.github/workflows/ci.yml` pins rustc 1.98.1 (components `rustfmt` and `clippy`) and installs apt package `protobuf-compiler`. This repo has no `rust-toolchain.toml`. The helper always runs `rustup run 1.98.1` and does not change the default toolchain.

The workspace `Cargo.toml` path dependencies need three directories beside this repo. `morpheum-proto` is required because `morpheum-primitives` depends on it.

```
parent/
  morpheum-signing/
  cryptogram/
  morpheum-primitives/
  morpheum-proto/
```

When a sibling `Cargo.toml` is missing, doctor prints `sibling_<name>=missing path=...` and `doctor=NOT_READY`. Stop. Do not clone the sibling from this skill, and do not add a stub crate.

Two run ids can build at once. Each id owns `/tmp/verify-morpheum-signing-<run-id>`. The runs share the cargo registry cache only. Drive only a run id you launched.

`mormcore` is not a dependency of this skill.

## Launch

Set `VERIFY_RUN_ID`, then launch.

```bash
export VERIFY_RUN_ID=20261030T000000Z-native-sign
.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh launch
```

A run id matches `^[A-Za-z0-9][A-Za-z0-9._-]{0,80}$`.

Ready means all of the following.

- The three sibling `Cargo.toml` files exist.
- `rustup run 1.98.1 cargo build -p morpheum-signing-native --examples` exits 0.
- `CARGO_TARGET_DIR` is `/tmp/verify-morpheum-signing-$VERIFY_RUN_ID/target`.
- These binaries exist and are executable: `target/debug/examples/native`, `agent`, and `agent_with_claim_verification`.
- The cargo log contains a Finished line for the dev profile (`unoptimized + debuginfo`).
- `evidence/$VERIFY_RUN_ID/launch/result.txt` is the single line `PASS`.
- The helper stdout contains `launch=OK`.

Launch installs rustc 1.98.1 when that toolchain is missing.

Launch installs apt packages `protobuf-compiler`, `pkg-config`, and `libssl-dev` when `protoc` is missing or `pkg-config --exists openssl` fails.

Launch refuses to build when `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` is set. Launch does not pass `--locked`. Launch does not pass `--no-default-features`. The default `full` feature set compiles `bip39` and `tokio` for these examples.

The helper copies `Cargo.lock` aside and writes that copy back before launch returns. A failed build restores the file too.

A second launch of the same run id builds again into the same target dir and rewrites `launch/`. That is safe.

## Doctor

Doctor only reads the checkout and writes a transcript. Run it before launch, and again when a drive fails, before you retry.

```bash
export VERIFY_RUN_ID=20261030T000000Z-native-sign
.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh doctor
```

`doctor=OK` means all of the following.

- The workspace `Cargo.toml` lists `crates/native`.
- `crates/native/Cargo.toml` names package `morpheum-signing-native`.
- `native.rs`, `agent.rs`, `agent_with_claim_verification.rs`, and `Cargo.lock` exist.
- The four `path = "../..."` entries in the workspace `Cargo.toml` resolve to files on disk.
- Sibling manifests for `cryptogram`, `morpheum-primitives`, and `morpheum-proto` exist.
- `rustup` is on `PATH`.

`toolchain_installed=no`, `protoc=missing`, and `openssl=missing` still leave `doctor=OK`. Launch installs them. `doctor=NOT_READY` means do not drive.

When `VERIFY_RUN_ID` is set, the transcript is `evidence/$VERIFY_RUN_ID/doctor/transcript.txt` and `doctor/result.txt` is `PASS` or `FAIL`.

Fmt, clippy, `cargo test`, and the wasm32 clippy leg are CI checks. They are not a drive.

## Drive

Drive one feature per command. The seeded proof is `drive-native-sign`. It executes the built `native` binary. It does not run `cargo test`.

```bash
export VERIFY_RUN_ID=20261030T000000Z-native-sign
.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh drive-native-sign
```

That binary is the one launch built with `cargo build -p morpheum-signing-native --examples`. The same program is `cargo run -p morpheum-signing-native --example native`. PASS reads the binary's stdout and exit code.

`native-sign` is `PASS` only when all of the following hold.

- The process exits 0.
- Stdout contains the exact line `Transaction signed successfully!`.
- Stdout contains a `TxHash` line whose value is 64 lowercase hex characters and is not 64 zeros.
- Stdout contains a `Raw bytes len` line whose integer is at least 1.
- The binary sha256 matches `launch/binaries.txt` for example `native`.
- No labeled 64-hex private key appears in stdout or stderr.

The other commands are `drive-mnemonic-native`, `drive-agent-claim`, and `drive-agent-claim-verify`. Recipes are in `features/`. A `native-sign` PASS does not cover the other files. Run doctor again after a failed drive, before you retry.

There is no `examples/evm.rs`. Do not treat a `cargo test` of `EvmSigner` as a drive.

## Evidence

Proof files live in `.cursor/skills/verify-morpheum-signing/evidence/$VERIFY_RUN_ID/` and stay after cleanup.

| Path | Contents |
| --- | --- |
| `doctor/transcript.txt` | Doctor lines, including `doctor=OK` or `doctor=NOT_READY` |
| `doctor/result.txt` | `PASS` or `FAIL` |
| `launch/command.txt` | The cargo build invocation and `CARGO_TARGET_DIR` |
| `launch/stdout.txt`, `launch/stderr.txt`, `launch/exit-code.txt` | The build |
| `launch/binaries.txt` | Example name, sha256, and absolute path |
| `launch/summary.txt` | `launch=OK` or `launch=FAIL`, rustc version, protoc version |
| `launch/result.txt` | `PASS` or `FAIL` |
| `<feature>/command.txt` | Feature id, example name, and the binary path |
| `<feature>/stdout.txt`, `<feature>/stderr.txt`, `<feature>/exit-code.txt` | The example process |
| `<feature>/binary-path.txt`, `<feature>/binary-sha256.txt` | The binary that ran |
| `<feature>/checks.txt` | Each stdout check |
| `<feature>/result.txt` | The single line `PASS` or `FAIL` |
| `result.txt` | The same line as the feature just driven |

`<feature>` for the seeded drive is `native-sign`. A passing seeded drive has `result.txt` and `native-sign/result.txt` equal to `PASS`.

The helper redacts GitHub tokens, `x-access-token` secrets, and values labeled `priv`, `private_key`, `secret_key`, or `priv_hex` that are 64 hex characters. The public example constants (seed `[42u8; 32]`, trading-key seed `[99u8; 32]`, and the BIP-39 abandon mnemonic) may remain when an example prints them. A labeled 64-hex private key forces `FAIL`.

These files are the proof. A green `cargo test` without this stdout is not a drive.

## Cleanup

Cleanup deletes `/tmp/verify-morpheum-signing-$VERIFY_RUN_ID` only. The path must start with `/tmp/verify-morpheum-signing-` and must not sit inside the repo. Cleanup does not delete `evidence/`, does not delete other run ids, and does not kill processes by name. Launch and drive are foreground processes. Do not clean up while one of them is still running.

```bash
export VERIFY_RUN_ID=20261030T000000Z-native-sign
.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh cleanup
```

Cleanup is safe to repeat after the scratch directory is already gone. A later drive of the same id needs a new launch, because the binaries lived in scratch.

After cleanup, confirm both of the following.

```bash
test ! -d "/tmp/verify-morpheum-signing-$VERIFY_RUN_ID"
grep -qx PASS ".cursor/skills/verify-morpheum-signing/evidence/$VERIFY_RUN_ID/result.txt"
grep -qx PASS ".cursor/skills/verify-morpheum-signing/evidence/$VERIFY_RUN_ID/native-sign/result.txt"
```

## Helpers

| Command | Role |
| --- | --- |
| `doctor` | Read-only checkout check. Writes evidence only when `VERIFY_RUN_ID` is set. |
| `launch` | Install rustc 1.98.1 and protoc if needed, then `cargo build -p morpheum-signing-native --examples`. Restores `Cargo.lock`. |
| `drive-native-sign` | Run the built `native` example. This is the seeded proof. |
| `drive-mnemonic-native` | Run the same `native` binary and require the mnemonic account line. |
| `drive-agent-claim` | Run the built `agent` example. |
| `drive-agent-claim-verify` | Run the built `agent_with_claim_verification` example. |
| `cleanup` | Delete `/tmp/verify-morpheum-signing-$VERIFY_RUN_ID`. Leave evidence. |

```bash
.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh --help
```

When the examples or the CI pin change, re-check this skill with `/maintain-verification-skill`.
