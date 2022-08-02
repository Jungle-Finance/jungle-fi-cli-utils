use std::fs;
use std::path::PathBuf;
use std::str::from_utf8;
use anchor_cli::config::Manifest;
use serde::{Serialize, Deserialize};
use std::process::{Command, Stdio};

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlTestMetadata {
    address: String,
}

/// Need to specify paths to all the programs in the repo.
/// Anchor test will attempt to look for a "metadata" field on the IDL containing an address,
/// and use that address to print transaction logs to `.anchor/program-logs`.
pub fn setup_anchor_program_log(program_list: &[(String, PathBuf)]) -> anyhow::Result<()> {
    for (address, path) in program_list {
        let cargo = Manifest::from_path(&path.join("Cargo.toml"))?;
        let version = cargo.version();
        let idl = anchor_syn::idl::file::parse(
            path.join("src/lib.rs"),
            version,
            true,
            false,
            false,
        )?;
        if let Some(mut idl) = idl {
            idl.metadata = Some(serde_json::to_value(IdlTestMetadata { address: address.clone() })?);
            let idl_out = PathBuf::from("target/idl")
                .join(&idl.name)
                .with_extension("json");
            let idl_json = serde_json::to_string_pretty(&idl)?;
            fs::write(idl_out, idl_json)?;
        }
    }
    Ok(())
}

pub fn execute_localnet_with(
    accounts: &Vec<Vec<String>>,
    cloned_programs: &Vec<(String, String)>,
    project_programs: &Vec<(String, PathBuf)>,
    extra_args: &Vec<String>,
) -> anyhow::Result<()> {
    let mut args = vec![];
    for act in accounts {
        args.extend(act.clone());
    }
    for (addr, filepath) in cloned_programs {
        args.extend(vec!["--bpf-program".to_string(), addr.clone(), filepath.clone()]);
    }
    args.extend(extra_args.clone());
    setup_anchor_program_log(&project_programs)?;
    execute_localnet_with_extra_accounts(&args)?;
    Ok(())
}

pub fn execute_localnet_with_extra_accounts(
    args: &Vec<String>,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("solana-test-validator");
    for item in args {
        cmd.arg(item);
    }
    let child = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let output = child.wait_with_output()?;
    let out = from_utf8(&output.stdout)?;
    println!("{:?}", out);
    Ok(())
}