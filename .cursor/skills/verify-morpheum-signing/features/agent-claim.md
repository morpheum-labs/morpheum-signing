# Agent claim

A caller builds a `TradingKeyClaim`, wraps it in an `AgentSigner`, and signs with `agent(signer)`. The example prints the claim sub-range size and the signed transaction.

## Sub-features

- `agent-banner` prints `Morpheum AI Agent Signing Example`.
- `agent-sub-range` prints `Claim sub-range size: 1000` for nonce bounds 1000 and 2000.
- `agent-sign-ok` prints `Agent transaction signed successfully with TradingKey + VC claim!`.
- `agent-txhash` prints a 64-hex `TxHash` and a positive `Raw bytes len`.

## How to get to it (user POV)

- Build the claim with `VcClaimBuilder` as `crates/native/examples/agent.rs` does. Issuer and subject are `AccountId([1u8; 32])`. Permissions are `1 << 0`. The nonce sub-range is 1000 to 2000. The signature is `Signature::Ed25519([1u8; 64])`, a public placeholder in that file.
- Call `AgentSigner::new` with trading-key seed `[99u8; 32]`, that agent id, and the claim.
- Call `agent(signer).chain_id("morpheum-test-1").with_genesis_hash([0x5A_u8; 32])`, set the memo from the example, add the market `Any`, call `with_trading_key_claim`, and `sign().await`.
- Read `Claim sub-range size`, the success line, `TxHash`, and `Raw bytes len`.

## Driving it with verify-morpheum-signing

Preconditions:

- `verify-morpheum-signing.sh doctor` prints `doctor=OK` for this `VERIFY_RUN_ID`.
- `verify-morpheum-signing.sh launch` prints `launch=OK`.
- `launch/binaries.txt` lists example `agent`.

1. Run `.cursor/skills/verify-morpheum-signing/scripts/verify-morpheum-signing.sh drive-agent-claim`.
2. Require exit code 0.
3. Require the exact line `Agent transaction signed successfully with TradingKey + VC claim!`.
4. Require the exact line `  Claim sub-range size: 1000`.
5. Require the same `TxHash` spacing and `Raw bytes len` spacing as [Native sign](./native-sign.md).
6. Require `evidence/$VERIFY_RUN_ID/agent-claim/result.txt` to be the single line `PASS`.

## Gotchas

- The signature bytes in the example are the placeholder `[1u8; 64]`, not a production issuer signature. This drive proves the example signs. It does not prove curve verification of that signature. That check is [Agent claim verification](./agent-claim-verify.md), and `verify()` there still does not check the ed25519 equation.
- `sub_range_size` is `nonce_sub_range_end - nonce_sub_range_start`. The example uses 1000 and 2000, so the line is `1000`. A different range that still signs is not this drive.
- Claim expiry uses `SystemTime::now()`. The `TxHash` changes between runs. The helper checks hash shape, not a golden hash.
- The trading-key seed `[99u8; 32]` is a public constant in the example. Do not replace it with a live key.
- Cleanup deletes the binary. Launch again before another drive of the same run id.
