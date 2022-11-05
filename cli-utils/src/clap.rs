use anyhow::{anyhow, Result};
use clap::parser::ArgMatches;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::signature::Signer;

/// Provides for clearer error messaging when using [Pubkey] type in clap args.
pub fn pubkey_arg(pubkey: &str) -> Result<Pubkey> {
    Pubkey::from_str(pubkey).map_err(
        |e| anyhow!("invalid pubkey: {}", e.to_string())
    )
}

/// Returns a pubkey using either its string representation,
/// or reading it as a signer path and retaining only that signer's public key.
/// This is useful when you happen to be in a place where you have more direct access
/// to a keypair path than the actual public key.
pub fn pubkey_or_signer_path(input: &str, matches: &ArgMatches) -> Result<Pubkey> {
    if let Ok(pubkey) = Pubkey::from_str(input) {
        Ok(pubkey)
    } else {
        let mut wallet_manager = None;
        let signer = signer_from_path(
            matches,
            input,
            "keypair",
            &mut wallet_manager,
        ).map_err(
            |e| anyhow!("invalid pubkey or signer path {}: {}", input, e.to_string())
        )?;
        Ok(signer.pubkey())
    }
}

/// Branch over the possible ways that signers can be specified via user input.
/// This basically does what `-k/--keypair` does, on a specific input string,
/// with disregard to filesystem configuration. It is useful for situations
/// where additional signers may be specified, e.g. grinding for an address and using
/// it as a signer when creating a multisig account.
pub fn parse_signer(matches: &ArgMatches, path: &str) -> anyhow::Result<Box<dyn Signer>> {
    let mut wallet_manager = None;
    let signer = signer_from_path(
        matches,
        path,
        "keypair",
        &mut wallet_manager,
    ).map_err(|e| anyhow!("Could not resolve signer: {:?}", e))?;
    Ok(signer)
}
