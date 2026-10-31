# Agent claim verification

A caller builds a `TradingKeyClaim`, runs `claim.verify` against the issuer public key, and signs with the claim embedded in `SignerInfo.signing_options`.

## Sub-features

- `claim-verify-passed` prints `Claim verification: PASSED`.
- `claim-embedded` prints `Claim embedded: YES (in SignerInfo.signing_options)`.
- `claim-sign-ok` prints `Transaction signed successfully with verified claim!` plus a 64-hex `TxHash` and a positive `Raw bytes len`.

## How to get to it (user POV)

- Create the issuer with `NativeSigner::from_seed(&[42u8; 32])` and the agent id with `NativeSigner::from_seed(&[99u8; 32])`, as `crates/native/examples/agent_with_claim_verification.rs` does.
- Build the claim with issuer and subject set to those account ids, permissions `0x01`, nonce sub-range 5000 to 6000, and `Signature::Ed25519([1u8; 64])`.
- Call `claim.verify(now_secs, &issuer_pubkey)`. On success the example prints `Claim verification: PASSED`. On failure it prints `Claim verification: FAILED` and still continues.
- Sign with `agent(agent_signer)` and `with_trading_key_claim`. Then read whether `signing_options` is present.

## Driving it with verify-morpheum-signing

Preconditions:

- `verify-morpheum-signing.sh doctor` prints `doctor=OK` for this `VERIFY_RUN_ID`.
- `verify-morpheum-signing.sh launch` prints `launch=OK`.
- `launch/binaries.txt` lists example `agent_with_claim_verification`.

1. Run `.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh drive-agent-claim-verify`.
2. Require exit code 0.
3. Require the exact line `  Claim verification: PASSED`.
4. Require the exact line `Transaction signed successfully with verified claim!`.
5. Require the exact line `  Claim embedded: YES (in SignerInfo.signing_options)`.
6. Require `  TxHash        : ` plus 64 lowercase hex characters, and `  Raw bytes len : ` plus a positive integer. The gaps are shorter than in `examples/native.rs`.
7. Fail the drive when stdout contains `Claim verification: FAILED`.
8. Require `evidence/$VERIFY_RUN_ID/agent-claim-verify/result.txt` to be the single line `PASS`.

## Gotchas

- `TradingKeyClaim::verify` checks expiry, nonce range, a non-zero signature, and that `claim.issuer` matches `issuer_pubkey.to_account_id()`. It does not verify the ed25519 signature. `PASSED` means those checks succeeded. Chain-side curve verification is outside this example.
- The example keeps going after `FAILED` and can still exit 0. Exit 0 without `PASSED` is `FAIL` for this drive. Do not drop the `PASSED` line to match that continuation.
- The signature in the file is the public placeholder `[1u8; 64]`. It is non-zero, so the structural check accepts it. Replacing it with an all-zero signature makes `verify` fail. Leave the example unchanged.
- `TxHash` spacing here is eight spaces between the word and the colon, not the ten spaces used by `examples/native.rs` and `examples/agent.rs`.
- Claim expiry uses `SystemTime::now()`. The hash is not golden-pinned.
- Cleanup deletes the binary. Launch again before another drive of the same run id.
