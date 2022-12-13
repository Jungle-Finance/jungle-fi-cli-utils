mod parse_keypair_from_path;

use std::str::FromStr;
use anchor_client::Cluster;
use solana_cli_config::Config;
use solana_sdk::signature::Keypair;
use log::warn;
use anyhow::anyhow;

pub use crate::cli::parse_keypair_from_path::keypair_from_path;

const LOCALNET_URL: &str = "http://localhost:8899";

/// Return a url [String] based on an optional url or [solana_cli_config::Config] object.
/// Passing [None] to both arguments will fetch the config file and resolve from there.
pub fn resolve_url(
    url: &Option<String>,
    config: Option<&Config>,
) -> anyhow::Result<String> {
    // Prioritize the URL, if passed in.
    if let Some(url) = url.clone() {
        return Ok(Cluster::from_str(&url)?.url().to_string());
    }
    // Otherwise, call back to the config file.
    // Find the config file (or create a default one), and use the supplied
    // RPC URL to create the [anchor_client::cluster::Cluster].
    if let Some(config) = config {
        return Ok(config.json_rpc_url.clone());
    }
    let config = get_solana_cli_config().unwrap_or(
        {
            warn!("No config file found or url provided, defaulting to localnet");
            println!("No config file found or url provided, defaulting to localnet");
            let mut config = Config::default();
            config.json_rpc_url = LOCALNET_URL.to_string();
            config
        }
    );
    Ok(config.json_rpc_url)
}

/// Return a [solana_sdk::signer::Keypair] based on an optional keypair path
/// or [solana_cli_config::Config] object.
/// Passing [None] to both arguments will fetch the config file and resolve from there.
pub fn resolve_keypair(
    keypair_path: &Option<String>,
    config: Option<&Config>,
) -> anyhow::Result<Box<Keypair>> {
    if let Some(keypair_path) = keypair_path {
        return keypair_from_path(keypair_path);
    }
    if let Some(config) = config {
        return keypair_from_path(&config.keypair_path);
    }
    let config = get_solana_cli_config().unwrap_or(
        {
            warn!("No config file found or -k/--keypair provided, defaulting to ~/.config/solana/id.json");
            println!("No config file found or -k/--keypair provided, defaulting to ~/.config/solana/id.json");
            let config = Config::default();
            config
        }
    );
    keypair_from_path(&config.keypair_path)
}


/// Load configuration from the standard Solana CLI config path.
/// Those config values are used as defaults at runtime whenever
/// keypair and/or url are not explicitly passed in.
/// This can possibly fail if there is no Solana CLI installed, nor a config file
/// at the expected location.
pub fn get_solana_cli_config() -> anyhow::Result<Config> {
    let config_file = solana_cli_config::CONFIG_FILE.as_ref()
        .ok_or_else(|| anyhow!("unable to determine a config file path on this OS or user"))?;
    Config::load(&config_file)
        .map_err(|e| anyhow!("unable to load config file: {}", e.to_string()))
}


#[cfg(test)]
mod tests {
    use solana_cli_config::Config;
    use super::*;

    #[test]
    fn test_resolve_url() {
        // Always use the passed URL.
        let url = resolve_url(&Some("foo".to_string()), None)
            .unwrap();
        assert_eq!(url, "foo".to_string());
        let mut config = Config::default();
        // Or use the config file
        config.json_rpc_url = "bar".to_string();
        let url = resolve_url(&None, Some(&config))
            .unwrap();
        assert_eq!(url, "bar".to_string());
        // Even if the config file was passed in, we use the url passed in.
        let url = resolve_url(&Some("foo".to_string()), Some(&config))
            .unwrap();
        assert_eq!(url, "foo".to_string());
    }

    #[test]
    fn test_resolve_keypair() {
        let path1 = "test/test-keypair.json";
        let path2 = "test/test-keypair2.json";
        let keypair1 = solana_sdk::signer::keypair::read_keypair_file(
            path1
        ).unwrap();
        let keypair2 = solana_sdk::signer::keypair::read_keypair_file(
            path2
        ).unwrap();
        // Always use the passed URL.
        let keypair = resolve_keypair(&Some(path1.to_string()), None)
            .unwrap();
        assert_eq!(*keypair, keypair1);
        let mut config = Config::default();
        // Or use the config file
        config.keypair_path = path2.to_string();
        let keypair = resolve_keypair(&None, Some(&config))
            .unwrap();
        assert_eq!(*keypair, keypair2);
        // Even if the config file was passed in, we use the url passed in.
        let keypair = resolve_keypair(&Some(path1.to_string()), Some(&config))
            .unwrap();
        assert_eq!(*keypair, keypair1);
    }
}
