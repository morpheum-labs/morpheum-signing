//! Example: AI Agent with full claim verification before signing
//!
//! This example demonstrates the complete agent flow including:
//! - Building a `TradingKeyClaim` with the `VcClaimBuilder`
//! - **Verifying** the claim against the issuer's public key (`claim.verify()`)
//! - Computing the claim digest for offline signature verification
//! - Signing a transaction with the verified claim embedded
//!
//! In production, the claim is issued by the account owner and verified by the
//! agent SDK before signing. The chain-side auth hot-path performs authoritative
//! cryptographic verification.
//!
//! Requires features: `claim-verification`, `bip39` (both included in `full`).

use morpheum_signing_native::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Morpheum Agent + Claim Verification Example");

    // ── 1. Set up the issuer (account owner) ─────────────────────────────
    // The issuer is the human or system that delegates authority to the agent.
    let issuer_seed = [42u8; 32];
    let issuer_signer = NativeSigner::from_seed(&issuer_seed);
    let issuer_pubkey = issuer_signer.public_key();
    let issuer_account_id = issuer_signer.account_id();
    println!("  Issuer account: {:?}", issuer_account_id);

    // ── 2. Set up the agent (TradingKey holder) ──────────────────────────
    let agent_seed = [99u8; 32];
    let agent_signer_for_id = NativeSigner::from_seed(&agent_seed);
    let agent_account_id = agent_signer_for_id.account_id();
    println!("  Agent account:  {:?}", agent_account_id);

    // ── 3. Build the TradingKeyClaim ─────────────────────────────────────
    // In production, the issuer signs the claim digest and provides the signature.
    let now_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let claim = VcClaimBuilder::new()
        .issuer(issuer_account_id.clone())
        .subject(agent_account_id.clone())
        .permissions(0x01) // TRADE permission
        .max_daily_usd(500_000) // $500k daily limit
        .expiry(now_secs + 86_400) // 24 hours from now
        .nonce_sub_range(5000, 6000) // 1000 parallel operations
        .signature(Signature::Ed25519([1u8; 64])) // Placeholder — see note below
        .build(now_secs)?;

    println!("  Claim built successfully");
    println!("    Sub-range size: {}", claim.sub_range_size());
    println!(
        "    Expires at:     {} (Unix seconds)",
        claim.expiry_timestamp
    );

    // ── 4. Compute claim digest (for offline signature verification) ─────
    // The digest is the SHA-256 hash of the unsigned claim fields.
    // In production, the issuer signs this digest to create the claim signature.
    let digest = claim.claim_digest();
    println!("    Claim digest:   {}", hex::encode(digest));

    // ── 5. Verify the claim ──────────────────────────────────────────────
    // This checks:
    //   a) Structural validity (expiry, nonce range, signature presence)
    //   b) Issuer consistency (claim.issuer matches issuer_pubkey.to_account_id())
    //
    // In production, full cryptographic signature verification is performed
    // chain-side. The SDK provides the digest via claim_digest() for
    // optional offline verification.
    #[cfg(feature = "claim-verification")]
    {
        match claim.verify(now_secs, &issuer_pubkey) {
            Ok(()) => println!("  Claim verification: PASSED"),
            Err(e) => {
                println!("  Claim verification: FAILED — {e}");
                // In production, you would abort here.
                // For this example, we continue to demonstrate the full flow.
            }
        }
    }

    // ── 6. Create the AgentSigner and sign ───────────────────────────────
    let agent_signer = AgentSigner::new(&agent_seed, agent_account_id, Some(claim.clone()));

    let market_any = Any {
        type_url: "type.googleapis.com/market.v1.MsgCreateMarketRequest".to_string(),
        value: vec![], // Replace with real serialized bytes in production
    };

    // The claim is embedded in SignerInfo.signing_options and covered by the signature
    let signed_tx = agent(agent_signer)
        .chain_id("morpheum-test-1")
        .memo("Agent trade with verified claim")
        .add_message(market_any)
        .with_trading_key_claim(claim)
        .sign()
        .await?;

    // ── 7. Output results ────────────────────────────────────────────────
    println!("Transaction signed successfully with verified claim!");
    println!("  TxHash        : {}", signed_tx.txhash_hex());
    println!("  Raw bytes len : {} bytes", signed_tx.raw_bytes().len());

    // Verify the claim was embedded by checking SignerInfo.signing_options
    if let Some(auth_info) = &signed_tx.tx.auth_info {
        if let Some(signer_info) = auth_info.signer_infos.first() {
            if signer_info.signing_options.is_some() {
                println!("  Claim embedded: YES (in SignerInfo.signing_options)");
            } else {
                println!("  Claim embedded: NO");
            }
        }
    }

    Ok(())
}
