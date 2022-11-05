/// Workaround to a bug in Anchor where the IDL files are not modified before text execution,
/// thus creating a problem with writing out program logs from `anchor test` to `.anchor/program-logs`
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use anchor_cli::config::Manifest;

/// Metadata fields to add to the IDL.
/// Anchor test localnet program logs expect the `address` field.
#[derive(Debug, Serialize, Deserialize)]
struct IdlTestMetadata {
    address: String,
}

/// This performs an action that normally only occurs during the beginning of `anchor test`,
/// and only if you choose *not* to use the `--skip-test-validator` flag.
/// `anchor test` will attempt to look for a "metadata" field on the IDL JSON file
/// containing an address, which it will use to print transaction logs to `.anchor/program-logs`.
///
/// Absence of this "metadata" field creates an error when testing is over.
///
/// This function a slice of tuples of Program ID and program crate path.
/// e.g. one element might be: `("EZ57....FHW2".to_string(), "programs/my-program".into())`.
/// It will then add the "address": "<program-id>" key-value pair to each IDL.
pub fn setup_anchor_test_program_log(program_list: &[(String, PathBuf)]) -> anyhow::Result<()> {
    for (address, path) in program_list {
        let cargo = Manifest::from_path(&path.join("Cargo.toml"))?;
        let version = cargo.version();
        let idl = anchor_syn::idl::file::parse(
            path.join("src/lib.rs"),
            version,
            false,
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