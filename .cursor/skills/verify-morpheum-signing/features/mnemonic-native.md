# Mnemonic native account

A caller derives a `NativeSigner` from the public BIP-39 abandon mnemonic and an empty passphrase, then signs with that signer. The example prints the derived account and the signed transaction.

## Sub-features

- `mnemonic-account` prints `Signer (from mnemonic) account:` plus the `AccountId` debug value.
- `mnemonic-sign-ok` prints `Transaction signed successfully!` from that signer.
- `mnemonic-txhash` prints a 64-hex `TxHash` and a positive `Raw bytes len`.

## How to get to it (user POV)

- Call `NativeSigner::from_mnemonic` with phrase `abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about` and passphrase `""`. That phrase is the public BIP-39 test vector already written in `crates/native/examples/native.rs`.
- The example prints `Signer (from seed) account:` first, from `NativeSigner::from_seed(&[42u8; 32])`, then the mnemonic account. The signer that reaches `sign()` is the mnemonic signer when the `bip39` feature is on.
- Sign with the same `native(signer)` chain as [Native sign](./native-sign.md). The account line is the extra observation.

## Driving it with verify-morpheum-signing

Preconditions:

- `verify-morpheum-signing.sh doctor` prints `doctor=OK` for this `VERIFY_RUN_ID`.
- `verify-morpheum-signing.sh launch` prints `launch=OK`.
- Launch used default features, so `bip39` is compiled into the `native` binary.

1. Run `.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh drive-mnemonic-native`.
2. Require exit code 0 and the same `TxHash` and `Raw bytes len` lines as [Native sign](./native-sign.md).
3. Require stdout to contain the prefix `  Signer (from mnemonic) account:`.
4. Require `evidence/$VERIFY_RUN_ID/mnemonic-native/result.txt` to be the single line `PASS`.

This command runs the same `native` binary as `drive-native-sign`. It does not prove a second program. It proves the mnemonic line was present.

## Gotchas

- The seed account line is not the mnemonic account. A binary built without `bip39` prints only the seed account, then signs with the seed signer. This drive fails in that case. Do not drop the mnemonic prefix to make it pass.
- The abandon phrase is a public test vector. The example does not print the phrase. Do not add a print of a different mnemonic.
- The helper does not pin the `AccountId` bytes. It requires the mnemonic label. Account debug format can change without this file naming a new expected id.
- `drive-native-sign` can pass on a future example that stops printing the mnemonic line. This file is the one that catches that.
- Cleanup deletes the binary. Launch again before another drive of the same run id.
