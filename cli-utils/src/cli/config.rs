/// The Solana CLI makes heavy use of a variety of design assumptions, including:
/// - A config file somewhere on the system.
/// - The ability to manually pass in `-k/--keypair` or `-u/--url`.
use std::str::FromStr;
use anchor_client::Cluster;
use anyhow::{anyhow, Result};
use log::warn;
use solana_cli_config::Config;
use anchor_client::solana_sdk::signature::Keypair;

use super::keypair_from_path;

pub const LOCALNET_URL: &str = "http://localhost:8899";

/// Return a cluster based on an optional url or [solana_cli_config::Config] object. If no
/// such object has been acquired, it can be left blank and fetched during
/// this function's call.
pub fn resolve_cluster(
    url: &Option<String>,
    config: Option<&Config>,
) -> Result<Cluster> {
    // Prioritize the URL, if passed in.
    if let Some(url) = url {
        Ok(Cluster::from_str(url)?)
    // Otherwise, call back to the config file.
    } else {
        // Find the config file (or create a default one), and use the supplied
        // RPC URL to create the [anchor_client::cluster::Cluster].
        let config_url = if let Some(config) = config {
            config.json_rpc_url.clone()
        } else {
            let config = get_solana_cli_config()?;
            config.json_rpc_url.clone()
        };
        Ok(Cluster::from_str(&config_url)?)
    }
}

/// Acquire a client and keypair either from command-line optional args, or
/// default to Solana CLI Config file.
/// Since there is no `usb` or `pubkey` path, we don't need an [ArgMatches]
/// and as a result, this function's return type becomes concrete, and
/// its argument signature gets simpler.
/// In the spirit of malfeasance, when all else fails, the last resort for cluster is localnet.
pub fn cluster_and_keypair_from_cli_config(
    keypair_path: &Option<String>,
    url: &Option<String>,
) -> Result<(Cluster, Box<Keypair>)> {
    let config = get_solana_cli_config();
    let config = config.unwrap_or_else(|_| {
        if keypair_path.is_none() {
            warn!("No config file found or -k/--keypair provided, defaulting to ~/.config/solana/id.json");
            println!("No config file found or -k/--keypair provided, defaulting to ~/.config/solana/id.json");
        }
        if url.is_none() {
            warn!("No config file found or -u/--url provided, defaulting to localnet");
            println!("No config file found or -u/--url provided, defaulting to localnet");
        }
        let mut config = Config::default();
        config.json_rpc_url = LOCALNET_URL.to_string();
        config
    });
    let cluster = resolve_cluster(
        url,
        Some(&config)
    )?;
    let keypair_path = if let Some(path) = keypair_path {
        path
    } else {
      &config.keypair_path
    };
    let keypair = keypair_from_path(
        keypair_path,
    )?;
    Ok((cluster, keypair))
}


/// Load configuration from the standard Solana CLI config path.
/// Those config values are used as defaults at runtime whenever
/// keypair and/or url are not explicitly passed in.
/// This can possibly fail if there is no Solana CLI installed, nor a config file
/// at the expected location.
pub fn get_solana_cli_config() -> Result<Config> {
    let config_file = solana_cli_config::CONFIG_FILE.as_ref()
        .ok_or_else(|| anyhow!("unable to determine a config file path on this OS or user"))?;
    Config::load(&config_file)
        .map_err(|e| anyhow!("unable to load config file: {}", e.to_string()))
}
