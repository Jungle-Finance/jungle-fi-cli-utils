/// Copied from Anchor `anchor-cli` crate.
use std::collections::HashSet;

use std::fs;
use std::fs::File;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::str::FromStr;
use anchor_cli::config::{Config, ConfigOverride, STARTUP_WAIT, TestConfig, TestValidator, WithPath};
use anchor_client::anchor_lang::idl::IdlAccount;
use anchor_client::Cluster;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Result};
use solana_program::bpf_loader_upgradeable;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signer;
use crate::idl::{IdlTestMetadata, on_chain_idl_account_data};
use crate::LocalnetAccount;

// Return the URL that solana-test-validator should be running on given the
// configuration
fn test_validator_rpc_url(test_validator: &Option<TestValidator>) -> String {
    match test_validator {
        Some(TestValidator {
                 validator: Some(validator),
                 ..
             }) => format!("http://{}:{}", validator.bind_address, validator.rpc_port),
        _ => "http://localhost:8899".to_string(),
    }
}

// Setup and return paths to the solana-test-validator ledger directory and log
// files given the configuration
fn test_validator_file_paths(test_validator: &Option<TestValidator>) -> (String, String) {
    let ledger_directory = match test_validator {
        Some(TestValidator {
                 validator: Some(validator),
                 ..
             }) => &validator.ledger,
        _ => ".anchor/test-ledger",
    };

    if !Path::new(&ledger_directory).is_relative() {
        // Prevent absolute paths to avoid someone using / or similar, as the
        // directory gets removed
        eprintln!("Ledger directory {} must be relative", ledger_directory);
        std::process::exit(1);
    }
    if !Path::new(&ledger_directory).exists() {
        fs::create_dir_all(&ledger_directory).unwrap();
    }
    (
        ledger_directory.to_string(),
        format!("{}/test-ledger-log.txt", ledger_directory),
    )
}

// Returns the solana-test-validator flags. This will embed the workspace
// programs in the genesis block so we don't have to deploy every time. It also
// allows control of other solana-test-validator features.
fn validator_flags(
    cfg: &WithPath<Config>,
    test_validator: &Option<TestValidator>,
) -> Result<Vec<String>> {
    let programs = cfg.programs.get(&Cluster::Localnet);

    // On-chain IDL accounts are written here.
    if !PathBuf::from("target/idl-account").exists() {
        fs::create_dir("target/idl-account")?;
    }

    let mut flags = Vec::new();
    for mut program in cfg.read_all_programs()? {
        let binary_path = program.binary_path().display().to_string();

        // Use the [programs.cluster] override and fallback to the keypair
        // files if no override is given.
        let address: Pubkey = programs
            .and_then(|m| m.get(&program.lib_name))
            .map(|deployment| Ok(deployment.address))
            .unwrap_or_else(|| program.pubkey())?;

        flags.push("--bpf-program".to_string());
        flags.push(address.clone().to_string());
        flags.push(binary_path);

        if let Some(idl) = program.idl.as_mut() {
            // Write the on-chain IDL account to a file and add it as an `--account` flag.
            let idl_account_data = on_chain_idl_account_data(
                &program.path.join("src/lib.rs").as_os_str().to_str().unwrap())?;
            let localnet_idl_act = LocalnetAccount::new(
                IdlAccount::address(&address),
                program.lib_name + "-account.json",
                IdlAccount {
                    authority: cfg.wallet_kp()?.pubkey(),
                    data: idl_account_data,
                },
            )
                .set_owner(address.clone());
            localnet_idl_act.write_to_validator_json_file("target/idl-account")?;
            flags.push("--account".to_string());
            flags.push(localnet_idl_act.address.to_string());
            flags.push(("target/idl-account/".to_string() + &localnet_idl_act.name)
                .as_str().to_string()
            );
            // Add program address to the IDL JSON file.
            // This is used during shutdown to log transactions.
            IdlTestMetadata { address: address.to_string() }.write_to_file(idl)?;
        }
    }

    if let Some(test) = test_validator.as_ref() {
        if let Some(genesis) = &test.genesis {
            for entry in genesis {
                let program_path = Path::new(&entry.program);
                if !program_path.exists() {
                    return Err(anyhow!(
                        "Program in genesis configuration does not exist at path: {}",
                        program_path.display()
                    ));
                }
                flags.push("--bpf-program".to_string());
                flags.push(entry.address.clone());
                flags.push(entry.program.clone());
            }
        }
        if let Some(validator) = &test.validator {
            let entries = serde_json::to_value(validator)?;
            for (key, value) in entries.as_object().unwrap() {
                if key == "ledger" {
                    // Ledger flag is a special case as it is passed separately to the rest of
                    // these validator flags.
                    continue;
                };
                if key == "account" {
                    for entry in value.as_array().unwrap() {
                        // Push the account flag for each array entry
                        flags.push("--account".to_string());
                        flags.push(entry["address"].as_str().unwrap().to_string());
                        flags.push(entry["filename"].as_str().unwrap().to_string());
                    }
                } else if key == "clone" {
                    // Client for fetching accounts data
                    let client = if let Some(url) = entries["url"].as_str() {
                        RpcClient::new(url.to_string())
                    } else {
                        return Err(anyhow!(
                    "Validator url for Solana's JSON RPC should be provided in order to clone accounts from   it"
                ));
                    };

                    let mut pubkeys = value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let address = entry["address"].as_str().unwrap();
                            Pubkey::from_str(address)
                                .map_err(|_| anyhow!("Invalid pubkey {}", address))
                        })
                        .collect::<anyhow::Result<HashSet<Pubkey>>>()?;

                    let accounts_keys = pubkeys.iter().cloned().collect::<Vec<_>>();
                    let accounts = client
                        .get_multiple_accounts_with_commitment(
                            &accounts_keys,
                            CommitmentConfig::default(),
                        )?
                        .value;

                    // Check if there are program accounts
                    for (account, acc_key) in accounts.iter().zip(accounts_keys) {
                        if let Some(account) = account {
                            if account.owner == bpf_loader_upgradeable::id() {
                                let upgradable: UpgradeableLoaderState = account
                                    .deserialize_data()
                                    .map_err(|_| anyhow!("Invalid program account {}", acc_key))?;

                                if let UpgradeableLoaderState::Program {
                                    programdata_address,
                                } = upgradable
                                {
                                    pubkeys.insert(programdata_address);
                                }
                            }
                        } else {
                            return Err(anyhow!("Account {} not found", acc_key));
                        }
                    }

                    for pubkey in &pubkeys {
                        // Push the clone flag for each array entry
                        flags.push("--clone".to_string());
                        flags.push(pubkey.to_string());
                    }
                } else {
                    // Remaining validator flags are non-array types
                    flags.push(format!("--{}", key.replace('_', "-")));
                    if let serde_json::Value::String(v) = value {
                        flags.push(v.to_string());
                    } else {
                        flags.push(value.to_string());
                    }
                }
            }
        }
    }
    Ok(flags)
}


fn stream_logs(config: &WithPath<Config>, rpc_url: &str) -> Result<Vec<Child>> {
    let program_logs_dir = ".anchor/program-logs";
    if Path::new(program_logs_dir).exists() {
        fs::remove_dir_all(program_logs_dir)?;
    }
    fs::create_dir_all(program_logs_dir)?;
    let mut handles = vec![];
    for program in config.read_all_programs()? {
        let mut file = File::open(&format!("target/idl/{}.json", program.lib_name))?;
        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        let idl: Idl = serde_json::from_slice(&contents)?;
        let metadata = idl.metadata.ok_or_else(|| {
            anyhow!(
                "Metadata property not found in IDL of program: {}",
                program.lib_name
            )
        })?;
        let metadata: IdlTestMetadata = serde_json::from_value(metadata)?;

        let log_file = File::create(format!(
            "{}/{}.{}.log",
            program_logs_dir, metadata.address, program.lib_name,
        ))?;
        let stdio = std::process::Stdio::from(log_file);
        let child = std::process::Command::new("solana")
            .arg("logs")
            .arg(metadata.address)
            .arg("--url")
            .arg(rpc_url)
            .stdout(stdio)
            .spawn()?;
        handles.push(child);
    }
    if let Some(test) = config.test_validator.as_ref() {
        if let Some(genesis) = &test.genesis {
            for entry in genesis {
                let log_file = File::create(format!("{}/{}.log", program_logs_dir, entry.address))?;
                let stdio = std::process::Stdio::from(log_file);
                let child = std::process::Command::new("solana")
                    .arg("logs")
                    .arg(entry.address.clone())
                    .arg("--url")
                    .arg(rpc_url)
                    .stdout(stdio)
                    .spawn()?;
                handles.push(child);
            }
        }
    }
    Ok(handles)
}

/// Run a `solana-test-validator` command according to a configuration specified
/// in an Anchor workspace or Test.toml file.
pub fn start_test_validator(
    cfg: &Config,
    test_validator: &Option<TestValidator>,
    flags: Option<Vec<String>>,
    test_log_stdout: bool,
) -> Result<Child> {
    //
    let (test_ledger_directory, test_ledger_log_filename) =
        test_validator_file_paths(test_validator);

    // Start a validator for testing.
    let (test_validator_stdout, test_validator_stderr) = match test_log_stdout {
        true => {
            let test_validator_stdout_file = File::create(&test_ledger_log_filename)?;
            let test_validator_sterr_file = test_validator_stdout_file.try_clone()?;
            (
                Stdio::from(test_validator_stdout_file),
                Stdio::from(test_validator_sterr_file),
            )
        }
        false => (Stdio::inherit(), Stdio::inherit()),
    };

    let rpc_url = test_validator_rpc_url(test_validator);

    let rpc_port = cfg
        .test_validator
        .as_ref()
        .and_then(|test| test.validator.as_ref().map(|v| v.rpc_port))
        .unwrap_or(solana_sdk::rpc_port::DEFAULT_RPC_PORT);
    if !portpicker::is_free(rpc_port) {
        return Err(anyhow!(
            "Your configured rpc port: {rpc_port} is already in use"
        ));
    }
    let faucet_port = cfg
        .test_validator
        .as_ref()
        .and_then(|test| test.validator.as_ref().and_then(|v| v.faucet_port))
        .unwrap_or(solana_faucet::faucet::FAUCET_PORT);
    if !portpicker::is_free(faucet_port) {
        return Err(anyhow!(
            "Your configured faucet port: {faucet_port} is already in use"
        ));
    }

    let mut validator_handle = std::process::Command::new("solana-test-validator")
        .arg("--ledger")
        .arg(test_ledger_directory)
        .arg("--mint")
        .arg(cfg.wallet_kp()?.pubkey().to_string())
        .args(flags.unwrap_or_default())
        .stdout(test_validator_stdout)
        .stderr(test_validator_stderr)
        .spawn()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    // Wait for the validator to be ready.
    let client = RpcClient::new(rpc_url);
    let mut count = 0;
    let ms_wait = test_validator
        .as_ref()
        .map(|test| test.startup_wait)
        .unwrap_or(STARTUP_WAIT);
    while count < ms_wait {
        let r = client.get_latest_blockhash();
        if r.is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
        count += 1;
    }
    if count == ms_wait {
        eprintln!(
            "Unable to get latest blockhash. Test validator does not look started. Check {} for errors.       Consider increasing [test.startup_wait] in Anchor.toml.",
            test_ledger_log_filename
        );
        validator_handle.kill()?;
        std::process::exit(1);
    }
    Ok(validator_handle)
}

pub fn localnet_from_test_config(test_config: TestConfig, flags: Vec<String>) -> Result<()> {
    for (_, test_toml) in &*test_config {
        // Copy the test suite into the Anchor [Config].
        // Set the startup_wait to zero, since it's irrelevant when we aren't running tests.
        let mut anchor_cfg = Config::discover(
            &ConfigOverride::default(),
        )?.unwrap();
        let mut test_validator = test_toml.test.clone();
        if let Some(inner) = test_validator {
            let mut with_no_wait = inner.clone();
            with_no_wait.startup_wait = 0;
            test_validator = Some(with_no_wait);
        } else {
            let mut with_no_wait = TestValidator::default();
            with_no_wait.startup_wait = 0;
            test_validator = Some(with_no_wait);
        }
        anchor_cfg.test_validator = test_validator;
        let with_path = &WithPath::new(
            anchor_cfg, PathBuf::from("./Anchor.toml"));
        // Gather the CLI flags
        let mut cfg_flags = validator_flags(
            &with_path, &test_toml.test)?;
        cfg_flags.extend(flags);
        // Start the validator
        let mut validator_handle = start_test_validator(
            &with_path,
            &test_toml.test,
            Some(cfg_flags),
            false,
        )?;

        let url = test_validator_rpc_url(&test_toml.test);
        let log_streams = stream_logs(
            &with_path,
            &url,
        );

        std::io::stdin().lock().lines().next().unwrap().unwrap();

        // Check all errors and shut down.
        if let Err(err) = validator_handle.kill() {
            println!(
                "Failed to kill subprocess {}: {}",
                validator_handle.id(),
                err
            );
        }

        for mut child in log_streams? {
            if let Err(err) = child.kill() {
                println!("Failed to kill subprocess {}: {}", child.id(), err);
            }
        }
        return Ok(())
    }
    Ok(())
}

pub fn start_localnet_from_test_toml(test_toml_path: &str, flags: Vec<String>) -> Result<()> {
    let path = PathBuf::from(test_toml_path);
    if !path.exists() {
        return Err(anyhow!("{} does not exist.", &test_toml_path));
    }
    if !path.is_file() {
        return Err(anyhow!("{} is not a file.", &test_toml_path));
    }
    let test_config = TestConfig::discover(&path.parent().unwrap(), vec![])?;
    if let Some(test_config) = test_config {
        localnet_from_test_config(test_config, flags)?;
        return Ok(());
    }
    Err(anyhow!("Failed to create a test configuration from {}", &test_toml_path))
}
