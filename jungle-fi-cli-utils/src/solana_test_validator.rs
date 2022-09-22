use std::fs;
use std::path::PathBuf;
use anchor_cli::config::Manifest;
use serde::{Serialize, Deserialize};

/// Metadata fields to add to the IDL.
/// Anchor test localnet program logs expect the `address` field.
#[derive(Debug, Serialize, Deserialize)]
pub struct IdlTestMetadata {
    address: String,
}

/// Anchor test will attempt to look for a "metadata" field on the IDL containing an address,
/// and use that address to print transaction logs to `.anchor/program-logs`.
/// This metadata field only gets added on a build or deployment step that we skip
/// during our testing, and as a result it causes an error. We need something similar
/// to this, but perhaps in a vanilla Python script.
pub fn setup_anchor_program_log(program_list: &[(String, PathBuf)]) -> anyhow::Result<()> {
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