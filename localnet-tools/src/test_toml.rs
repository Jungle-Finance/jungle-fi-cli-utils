use std::fs;
use anyhow::anyhow;
use anchor_cli::config::{_TestToml, _TestValidator, _Validator, AccountEntry, GenesisEntry, ScriptsConfig};
use crate::localnet_account::LocalnetAccount;


/// Standard Anchor test command. The [TestTomlGenerator.test_file_glob] is appended
/// to this and added to the `[script]` section of the `Test.toml` file under the name `"test"`.
const TEST_CMD_PREFIX: &str = "yarn run ts-mocha -p ./tsconfig.json -t 1000000";

/// Beginning of JS file, to construct `anchor.web3.PublicKey` instances.
const JS_ANCHOR_IMPORT: &str = "import * as anchor from \"@project-serum/anchor\";\n";
/// Save location for the JS file
const JS_IMPORT_FILE: &str = "accounts.ts";

/// Generates a `Test.toml` that sets up a localnet for testing, and provides
/// other convenient setup automation for complicated state saturation.
#[derive(Debug, Clone, Default)]
pub struct TestTomlGenerator {
    /// The directory where the Test.toml will exist.
    pub save_directory: String,
    /// If it's a test suite, this specifies the .ts/.js file(s) to execute.
    pub test_file_glob: Option<String>,
    /// Any accounts to pre-load to the test validator.
    pub accounts: Vec<LocalnetAccount>,
    /// Any programs to pre-load to the test validator.
    /// Tuples are of (Address, ProgramPath).
    pub programs: Vec<(String, String)>,
    /// Any settings for the test validator.
    pub validator_settings: Option<_Validator>,
    /// Relative paths to any other Test.toml files to extend the configuration.
    pub extends: Vec<String>,
    /// To ensure that the test validator has enough time to start up before tests begin.
    pub startup_wait: Option<i32>,
    pub shutdown_wait: Option<i32>,
}

impl TestTomlGenerator {
    pub fn build(&self) -> anyhow::Result<()> {
        self.write_accounts()?;
        self.write_js_import_file()?;
        self.write_toml()?;
        Ok(())
    }

    pub fn write_accounts(&self) -> anyhow::Result<()> {
        for act in &self.accounts {
            act.write_to_validator_json_file(&self.save_directory)?;
        }
        Ok(())
    }

    /// Create a file that allows for easy import of the files in this test suite.
    pub fn write_js_import_file(&self) -> anyhow::Result<()> {
        let mut script = vec![JS_ANCHOR_IMPORT.to_string()];
        script
            .extend(
                self.accounts
                    .iter()
                    .map(|act| act.js_import())
                    .collect::<Vec<String>>()
            );
        let script: String = script.join("\n");
        let save_to = self.save_directory.as_str().to_owned() + "/" + JS_IMPORT_FILE;
        fs::write(&save_to, script)
            .map_err(|e| anyhow!("Error writing to {}: {:?}", save_to, e))?;
        Ok(())
    }

    pub fn write_toml(&self) -> anyhow::Result<()> {
        // [[test.validator.account]] blocks
        let account_entries: Vec<AccountEntry> = self.accounts
            .iter()
            .map(|act| act.to_account_entry())
            .collect();
        let account_entries = if account_entries.is_empty() {
            None
        } else {
            Some(account_entries)
        };
        // [test] settings
        let validator = if let Some(settings) = self.validator_settings.clone() {
            _Validator {
                account: account_entries, ..settings
            }
        } else {
            _Validator {
                account: account_entries, ..Default::default()
            }
        };
        // Then add pre-loaded programs
        let mut test_validator = _TestValidator::default();
        test_validator.validator = Some(validator);
        let genesis_programs: Vec<GenesisEntry> = self.programs
            .iter()
            .map(|(addr, path)| GenesisEntry {
                address: addr.to_string(),
                program: path.to_string(),
            })
            .collect();
        let genesis_programs = if genesis_programs.is_empty() {
            None
        } else {
            Some(genesis_programs)
        };
        test_validator.startup_wait = self.startup_wait;
        test_validator.shutdown_wait = self.shutdown_wait;
        test_validator.genesis = genesis_programs;
        // Then add any extensions to other .toml files
        let extends = if self.extends.is_empty() {
            None
        } else {
            Some(self.extends.clone())
        };
        // Add a test block if necessary
        let scripts = if let Some(s) = self.test_file_glob.clone() {
            let mut test_scripts = ScriptsConfig::new();
            let test_script = format!("{} {}", TEST_CMD_PREFIX, s);
            test_scripts.insert("test".to_string(), test_script);
            Some(test_scripts)
        } else {
            None
        };
        // Write TOML to file.
        let test_toml = _TestToml {
            extends,
            test: Some(test_validator),
            scripts,
        };
        let toml_str_output = toml::to_string(&test_toml).unwrap();
        let save_to = self.save_directory.as_str().to_owned() + "/Test.toml";
        fs::write(&save_to, toml_str_output)
            .map_err(|e| anyhow!("Error writing to {}: {:?}", save_to, e))?;
        Ok(())
    }
}
