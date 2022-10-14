use std::borrow::Cow;
use anchor_client::anchor_lang::{AccountDeserialize, AccountSerialize};
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_sdk::account::{Account, AccountSharedData, ReadableAccount};
use solana_program::clock::Epoch;
use solana_account_decoder::UiAccount;
use std::fs::File;
use serde_json::json;
use anyhow::Result;
use solana_program::pubkey::{Pubkey};
use solana_program::system_program;
use inflector::Inflector;

/// Create account data wholecloth, from any type that implements
/// [anchor_lang::AccountSerialize] and [anchor_lang::AccountDeserialize].
pub trait GeneratedAccount {
    type T: AccountSerialize + AccountDeserialize;

    fn address(&self) -> Pubkey;

    fn generate(&self) -> Self::T;

    fn lamports(&self) -> u64 {
        1_000_000_000_000
    }

    fn owner(&self) -> Pubkey {
        system_program::id()
    }

    fn executable(&self) -> bool {
        false
    }

    fn rent_epoch(&self) -> Epoch {
        0
    }

    fn save_location(&self) -> String {
        format!("{}.json", self.address().to_string())
    }

    fn arg(&self) -> Vec<String> {
        vec!["--account".to_string(), self.address().to_string(), self.save_location()]
    }

    fn js_import(&self) -> String {
        js_test_import(&self.save_location())
    }

    // Can add this data type directly to a [TestValidatorGenesis] accounts to load.
    fn account_shared_data(&self) -> (Pubkey, AccountSharedData) {
        let data = self.generate();
        let mut buf = vec![];
        data.try_serialize(&mut buf).unwrap();
        (self.address(), Account {
            lamports: self.lamports(),
            owner: self.owner(),
            data: buf,
            executable: self.executable(),
            rent_epoch: self.rent_epoch(),
        }.into())
    }

    fn write_to_validator_json_file(&self) -> Result<()> {
        let account_data = self.generate();
        write_to_validator_json_file(
            &self.address(),
            &self.save_location(),
            self.lamports(),
            account_data,
            &self.owner(),
            self.executable(),
            self.rent_epoch(),
        )?;
        Ok(())
    }
}

/// Clone an account from a cluster, and optionally modify it.
/// Only works on account types that implement [anchor_lang::AccountSerialize]
/// and [anchor_lang::AccountDeserialize].
pub trait ClonedAccount {
    type T: AccountSerialize + AccountDeserialize;

    fn address(&self) -> Pubkey;

    fn save_location(&self) -> String {
        format!("{}.json", self.address().to_string())
    }

    fn arg(&self) -> Vec<String> {
        vec!["--account".to_string(), self.address().to_string(), self.save_location()]
    }

    fn js_import(&self) -> String {
        js_test_import(&self.save_location())
    }

    /// Default implementation performs no modification
    fn modify(&self, deserialized: Self::T) -> Self::T {
        deserialized
    }

    fn fetch_and_modify_data(&self, client: &RpcClient) -> Result<(Account, Self::T)> {
        let address = self.address();
        let info = client
            .get_account(&address)?;
        let deserialized = Self::T::try_deserialize(
            &mut info.data.as_slice())?;
        Ok((info, self.modify(deserialized)))
    }

    // Can add this data type directly to a [TestValidatorGenesis] accounts to load.
    fn account_shared_data(&self, client: &RpcClient) -> Result<(Pubkey, AccountSharedData)> {
        let (info, data) = self.fetch_and_modify_data(client)?;
        let mut buf = vec![];
        data.try_serialize(&mut buf).unwrap();
        Ok((self.address(), Account {
            lamports: info.lamports,
            owner: info.owner,
            data: buf,
            executable: info.executable,
            rent_epoch: info.rent_epoch,
        }.into()))
    }

    fn write_to_validator_json_file(&self, client: &RpcClient) -> Result<()> {
        let (info, modified) = self.fetch_and_modify_data(client)?;
        write_to_validator_json_file(
            &self.address(),
            &self.save_location(),
            info.lamports,
            modified,
            info.owner(),
            info.executable,
            info.rent_epoch,
        )?;
        Ok(())
    }
}

/// Takes any type that implements `anchor_lang::AccountSerialize`
/// and saves it as a JSON file at a specified filepath.
pub fn write_to_validator_json_file<T: AccountSerialize>(
    address: &Pubkey,
    save_location: &str,
    lamports: u64,
    account_data: T,
    owner: &Pubkey,
    executable: bool,
    rent_epoch: Epoch,
) -> Result<()> {
    let mut serialized = Vec::new();
    account_data.try_serialize(&mut serialized)?;
    let ui_act = crate::account::parsing::ui_account(
        lamports,
        &serialized,
        owner,
        executable,
        rent_epoch,
    );
    write_to_file(
        address,
        ui_act,
        save_location,
    )?;
    Ok(())
}

/// Writes to file in the format expected by `solana-test-validator`
/// and associated tooling.
pub fn write_to_file(
    address: &Pubkey,
    ui_act: UiAccount,
    save_location: &str,
) -> Result<()> {
    let address = address.to_string();
    let file = File::create(save_location)?;
    serde_json::to_writer_pretty(
        file,
        &json!({
                    "pubkey": address,
                    "account": &ui_act,
                }),
    )?;
    Ok(())
}

fn basename(path: &str, sep: char) -> String {
    let mut pieces = path.rsplit(sep);
    match pieces.next() {
        Some(p) => p.to_string(),
        None => path.to_string(),
    }
}

fn js_test_import(location: &str) -> String {
    let location = if !location.ends_with(".json") {
        let (_, location) = location.split_at(location.len()-5);
        location.to_string()
    } else {
        location.to_string()
    };
    let name = basename(&location, '/');
    let (name, _) = name.split_at(name.len() - 5);
    let name = name.to_string().to_camel_case();
    format!("import * as {}Json from \"../{}\";\nexport const {} = new anchor.web3.PublicKey({}Json.pubkey);", &name, &location, &name, &name)
}
