use std::net::SocketAddr;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::str::FromStr;
use anchor_client::solana_sdk;
use anchor_client::solana_sdk::signature::read_keypair_file;
use anchor_client::solana_sdk::signer::Signer;
use clap::ArgMatches;
use solana_program::pubkey::Pubkey;
use solana_test_validator::ProgramInfo;
use solana_clap_v3_utils::keypair::signer_from_path;

pub mod keypair_from_path;
pub mod config;

/// This is the code equivalent of `--bpf-program <ADDRESS> <FILEPATH>` on
/// the `solana-test-validator` command.
pub fn program_info(address: String, program: String) -> Result<ProgramInfo> {
    let address = address
        .parse::<Pubkey>()
        .or_else(|_| read_keypair_file(&address).map(|keypair| keypair.pubkey()))
        .map_err(|_| anyhow!("failed to read keypair file for program info {}", &address))?;

    let program_path = PathBuf::from(program);
    if !program_path.exists() {
        return Err(anyhow!(
            "Error: program file does not exist: {}",
            program_path.display()
        ));
    }

    Ok(ProgramInfo {
        program_id: address,
        loader: solana_sdk::bpf_loader::id(),
        program_path,
    })
}

/// Provides for clearer error messaging when using [Pubkey] type in clap args.
pub fn parse_pubkey(pubkey: &str) -> Result<Pubkey> {
    Pubkey::from_str(pubkey).map_err(
        |e| anyhow!("invalid pubkey: {}", e.to_string())
    )
}

/// Returns a pubkey from either its string representation, or a signer path.
/// Similar to [parse_signer] below, only difference is that we only care about
/// the public key with this function.
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
pub fn parse_signer(matches: &ArgMatches, path: &str) -> Result<Box<dyn Signer>> {
    let mut wallet_manager = None;
    let signer = signer_from_path(
        matches,
        path,
        "keypair",
        &mut wallet_manager,
    ).map_err(|e| anyhow!("Could not resolve signer: {:?}", e))?;
    Ok(signer)
}
