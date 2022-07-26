/// Solana test validator stripped of many of its configuration properties in lieu of
/// opinionated defaults.
/// The goal here is simply to complex structure to the loaded accounts and programs
/// in a manner that is easier to define and manage using `--account` and `--bpf-program` flags.
use std::str::FromStr;
use anchor_client::Cluster;
use clap::Parser;
use crossbeam::channel::Sender;
use {
    clap::{crate_name},
    log::*,
    solana_client::rpc_client::RpcClient,
    solana_core::tower_storage::FileTowerStorage,
    solana_faucet::faucet::{run_local_faucet_with_port, FAUCET_PORT},
    solana_rpc::{
        rpc::{JsonRpcConfig},
        rpc_pubsub_service::PubSubConfig,
    },
    solana_sdk::{
        account::AccountSharedData,
        clock::Slot,
        native_token::sol_to_lamports,
        pubkey::Pubkey,
        signature::{read_keypair_file, write_keypair_file, Keypair, Signer},
        system_program,
    },
    solana_streamer::socket::SocketAddrSpace,
    solana_test_validator::*,
    solana_validator::{
        admin_rpc_service, dashboard::Dashboard, ledger_lockfile, lock_ledger, println_name_value,
        redirect_stderr_to_file,
    },
    std::{
        collections::HashSet,
        fs, io,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::{Path, PathBuf},
        process::exit,
        sync::{Arc, RwLock},
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
};

/* 10,000 was derived empirically by watching the size
 * of the rocksdb/ directory self-limit itself to the
 * 40MB-150MB range when running `solana-test-validator`
 */
const DEFAULT_MAX_LEDGER_SHREDS: u64 = 10_000;

const DEFAULT_FAUCET_SOL: f64 = 1_000_000.;

#[derive(PartialEq)]
enum Output {
    Log,
    Dashboard,
}

#[derive(Debug, Parser)]
pub struct CliArgs {
    #[clap(long("ledger"), value_name("DIR"), default_value("test-ledger"))]
    ledger_path: String,
    #[clap(short, long)]
    reset: bool,
    #[clap(long, number_of_values=2, value_name="ADDRESS_OR_PATH BPF_PROGRAM.SO")]
    bpf_program: Option<Vec<String>>,
    #[clap(long, number_of_values=2, value_name="ADDRESS FILENAME.JSON")]
    account: Option<Vec<String>>,
    #[clap(short, long)]
    warp_slot: Option<u64>,
    #[clap(short('u'), long("url"))]
    json_rpc_url: Option<String>,
    #[clap(short('c'), long("clone"), value_name="ADDRESS", requires("json-rpc-url"))]
    clone_account: Option<Vec<String>>,
}

pub fn program_info(address: String, program: String) -> anyhow::Result<ProgramInfo> {
    let address = address
        .parse::<Pubkey>()
        .or_else(|_| read_keypair_file(&address).map(|keypair| keypair.pubkey()))
        .unwrap_or_else(|err| {
            println!("Error: invalid address {}: {}", &address, err);
            exit(1);
        });

    let program_path = PathBuf::from(program);
    if !program_path.exists() {
        println!(
            "Error: program file does not exist: {}",
            program_path.display()
        );
        exit(1);
    }

    Ok(ProgramInfo {
        program_id: address,
        loader: solana_sdk::bpf_loader::id(),
        program_path,
    })
}

pub fn custom_localnet(
    accounts: Vec<(Pubkey, AccountSharedData)>,
    programs: Vec<ProgramInfo>,
    clones: Vec<Pubkey>,
) {
    let default_rpc_port = solana_sdk::rpc_port::DEFAULT_RPC_PORT.to_string();
    let default_faucet_port = FAUCET_PORT.to_string();
    let default_limit_ledger_size = DEFAULT_MAX_LEDGER_SHREDS.to_string();
    let default_faucet_sol = DEFAULT_FAUCET_SOL.to_string();

    let cli_args = CliArgs::parse();

    let output = Output::Dashboard;

    let ledger_path = Path::new(&cli_args.ledger_path);

    let reset_ledger = cli_args.reset;

    if !ledger_path.exists() {
        fs::create_dir(&ledger_path).unwrap_or_else(|err| {
            println!(
                "Error: Unable to create directory {}: {}",
                ledger_path.display(),
                err
            );
            exit(1);
        });
    }

    let mut ledger_lock = ledger_lockfile(&ledger_path);
    let _ledger_write_guard = lock_ledger(&ledger_path, &mut ledger_lock);
    if reset_ledger {
        remove_directory_contents(&ledger_path).unwrap_or_else(|err| {
            println!("Error: Unable to remove {}: {}", ledger_path.display(), err);
            exit(1);
        })
    }
    solana_runtime::snapshot_utils::remove_tmp_snapshot_archives(&ledger_path);

    let validator_log_symlink = ledger_path.join("validator.log");

    let logfile = if output != Output::Log {
        let validator_log_with_timestamp = format!(
            "validator-{}.log",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let _ = fs::remove_file(&validator_log_symlink);
        symlink::symlink_file(&validator_log_with_timestamp, &validator_log_symlink).unwrap();

        Some(
            ledger_path
                .join(validator_log_with_timestamp)
                .into_os_string()
                .into_string()
                .unwrap(),
        )
    } else {
        None
    };
    let _logger_thread = redirect_stderr_to_file(logfile);

    info!("{} {}", crate_name!(), solana_version::version!());
    info!("Starting validator with: {:#?}", std::env::args_os());
    solana_core::validator::report_target_features();

    // TODO: Ideally test-validator should *only* allow private addresses.
    let socket_addr_space = SocketAddrSpace::new(/*allow_private_addr=*/ true);

    let cli_config = if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };


    let (mint_address, random_mint) =
        read_keypair_file(&cli_config.keypair_path)
            .map(|kp| (kp.pubkey(), false))
            .unwrap_or_else(|_| (Keypair::new().pubkey(), true));

    let rpc_port = u16::from_str(&default_rpc_port).unwrap();
    let enable_vote_subscription = false;
    let faucet_port = u16::from_str(&default_faucet_port).unwrap();

    let faucet_addr = Some(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        faucet_port,
    ));

    let mut programs_to_load = vec![];
    if let Some(values) = &cli_args.bpf_program {
        for address_program in values.chunks(2) {
            match address_program {
                [address, program] => {
                    let address = address
                        .parse::<Pubkey>()
                        .or_else(|_| read_keypair_file(address).map(|keypair| keypair.pubkey()))
                        .unwrap_or_else(|err| {
                            println!("Error: invalid address {}: {}", address, err);
                            exit(1);
                        });

                    let program_path = PathBuf::from(program);
                    if !program_path.exists() {
                        println!(
                            "Error: program file does not exist: {}",
                            program_path.display()
                        );
                        exit(1);
                    }

                    programs_to_load.push(ProgramInfo {
                        program_id: address,
                        loader: solana_sdk::bpf_loader::id(),
                        program_path,
                    });
                }
                _ => unreachable!(),
            }
        }
    }

    let mut accounts_to_load = vec![];
    if let Some(values) = &cli_args.account {
        for address_filename in values.chunks(2) {
            match address_filename {
                [address, filename] => {
                    let address = address.parse::<Pubkey>().unwrap_or_else(|err| {
                        println!("Error: invalid address {}: {:?}", address, err);
                        exit(1);
                    });

                    accounts_to_load.push(AccountInfo { address, filename });
                }
                _ => unreachable!(),
            }
        }
    }

    let json_rpc_url = cli_args.json_rpc_url.unwrap_or(Cluster::Mainnet.url().to_string());
    let cluster = Cluster::from_str(&json_rpc_url)
        .unwrap_or_else(|err|{
            println!(
                "Error: Invalid cluster URL {}: {}",
                json_rpc_url,
                err
            );
            exit(1);
        });
    let cluster_rpc_client = RpcClient::new(cluster.url());
    let mut accounts_to_clone: HashSet<_> = cli_args.clone_account
        .map(|accounts| accounts
            .iter()
            .map::<Pubkey, _>(|p| Pubkey::from_str(p).unwrap_or_else(|_| {
                    read_keypair_file(p)
                        .expect("read_keypair_file failed")
                        .pubkey()
                }))
            .collect()
        )
        .unwrap_or_default();
    for pubkey in clones {
        accounts_to_clone.insert(pubkey);
    }

    let warp_slot = cli_args.warp_slot.map(|s| Slot::from(s));

    let faucet_lamports = sol_to_lamports(f64::from_str(&default_faucet_sol).unwrap());
    let faucet_keypair_file = ledger_path.join("faucet-keypair.json");
    if !faucet_keypair_file.exists() {
        write_keypair_file(&Keypair::new(), faucet_keypair_file.to_str().unwrap()).unwrap_or_else(
            |err| {
                println!(
                    "Error: Failed to write {}: {}",
                    faucet_keypair_file.display(),
                    err
                );
                exit(1);
            },
        );
    }

    let faucet_keypair =
        read_keypair_file(faucet_keypair_file.to_str().unwrap()).unwrap_or_else(|err| {
            println!(
                "Error: Failed to read {}: {}",
                faucet_keypair_file.display(),
                err
            );
            exit(1);
        });
    let faucet_pubkey = faucet_keypair.pubkey();

    if let Some(faucet_addr) = &faucet_addr {
        let (sender, receiver): (Sender<Result<SocketAddr, String>>, _) = crossbeam::channel::unbounded();
        run_local_faucet_with_port(faucet_keypair, sender, None, faucet_addr.port());
        let _ = receiver.recv().expect("run faucet").unwrap_or_else(|err| {
            println!("Error: failed to start faucet: {}", err);
            exit(1);
        });
    }

    if TestValidatorGenesis::ledger_exists(&ledger_path) {
        println!("ledger already exists, not loading any programs or accounts, nor cloning accounts");
    } else if random_mint {
        println_name_value(
            "\nNotice!",
            "No wallet available. `solana airdrop` localnet SOL after creating one\n",
        );
    }

    let mut genesis = TestValidatorGenesis::default();
    genesis.max_ledger_shreds = Some(u64::from_str(&default_limit_ledger_size).unwrap());
    genesis.max_genesis_archive_unpacked_size = Some(u64::MAX);
    genesis.accounts_db_caching_enabled = true;

    let tower_storage = Arc::new(FileTowerStorage::new(ledger_path.to_path_buf()));

    let admin_service_post_init = Arc::new(RwLock::new(None));
    admin_rpc_service::run(
        &ledger_path,
        admin_rpc_service::AdminRpcRequestMetadata {
            rpc_addr: Some(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                rpc_port,
            )),
            start_progress: genesis.start_progress.clone(),
            start_time: SystemTime::now(),
            validator_exit: genesis.validator_exit.clone(),
            authorized_voter_keypairs: genesis.authorized_voter_keypairs.clone(),
            post_init: admin_service_post_init.clone(),
            tower_storage: tower_storage.clone(),
        },
    );
    let dashboard = if output == Output::Dashboard {
        Some(
            Dashboard::new(
                &ledger_path,
                Some(&validator_log_symlink),
                Some(&mut genesis.validator_exit.write().unwrap()),
            )
            .unwrap(),
        )
    } else {
        None
    };
    println!("It got here");

    genesis
        .ledger_path(&ledger_path)
        .tower_storage(tower_storage)
        .add_account(
            faucet_pubkey,
            AccountSharedData::new(faucet_lamports, 0, &system_program::id()),
        )
        .rpc_config(JsonRpcConfig {
            enable_rpc_transaction_history: true,
            faucet_addr,
            ..JsonRpcConfig::default_for_test()
        })
        .pubsub_config(PubSubConfig {
            enable_vote_subscription,
            ..PubSubConfig::default()
        })
        .bpf_jit(true)
        .rpc_port(rpc_port)
        .add_programs_with_path(&programs_to_load)
        .add_accounts_from_json_files(&accounts_to_load);

    if !accounts_to_clone.is_empty() {
        genesis.clone_accounts(
            accounts_to_clone,
            &cluster_rpc_client,
            false,
        );
    }

    for (addr, data) in accounts {
        genesis.add_account(addr, data);
    }
    genesis.add_programs_with_path(&programs);

    if let Some(warp_slot) = warp_slot {
        genesis.warp_slot(warp_slot);
    }

    match genesis.start_with_mint_address(mint_address, socket_addr_space) {
        Ok(test_validator) => {
            *admin_service_post_init.write().unwrap() =
                Some(admin_rpc_service::AdminRpcRequestMetadataPostInit {
                    bank_forks: test_validator.bank_forks(),
                    cluster_info: test_validator.cluster_info(),
                    vote_account: test_validator.vote_account_address(),
                });
            if let Some(dashboard) = dashboard {
                dashboard.run(Duration::from_millis(250));
            }
            test_validator.join();
        }
        Err(err) => {
            drop(dashboard);
            println!("Error: failed to start validator: {}", err);
            exit(1);
        }
    }
}

fn remove_directory_contents(ledger_path: &Path) -> Result<(), io::Error> {
    for entry in fs::read_dir(&ledger_path)? {
        let entry = entry?;
        if entry.metadata()?.is_dir() {
            fs::remove_dir_all(&entry.path())?
        } else {
            fs::remove_file(&entry.path())?
        }
    }
    Ok(())
}
