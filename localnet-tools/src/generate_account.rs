use anchor_client::anchor_lang::{AccountDeserialize, AccountSerialize};
use anchor_client::solana_sdk::account::{Account, AccountSharedData};
use solana_program::clock::Epoch;
use solana_account_decoder::UiAccount;
use std::fs::File;
use serde_json::json;
use anyhow::Result;
use solana_program::pubkey::Pubkey;
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
    let ui_act = crate::ui_account(
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

pub fn js_test_import(location: &str) -> String {
    //let mut location = &mut location.clone();
    let location = if !location.ends_with(".json") {
        let (_, location) = location.split_at(location.len()-5);
        location.to_string()
    } else {
        location.to_string()
    };
    let name = {
        let mut pieces = location.rsplit('/');
        match pieces.next() {
            Some(p) => p.to_string(),
            None => location.to_string(),
        }
    };
    // Cut off the ".json" part.
    let (name, _) = name.split_at(name.len() - 5);
    // Turn it into "camelCase" ending in "Json", e.g. i_mint.json -> iMintJson.
    let name = name.to_string().to_camel_case();
    // Output an import statement
    // and its subsequent extraction of the Typescript `PublicKey` object.
    format!("import * as {}Json from \"../{}\";\nexport const {} = new anchor.web3.PublicKey({}Json.pubkey);", &name, &location, &name, &name)
}
