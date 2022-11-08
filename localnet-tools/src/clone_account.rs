use anchor_client::anchor_lang::{AccountDeserialize, AccountSerialize};
use solana_program::pubkey::Pubkey;
use anchor_client::solana_client::rpc_client::RpcClient;
use solana_sdk::account::{Account, ReadableAccount};
use crate::generate_account;

/// Allows modification of cloned account data in its deserialized form
/// before being written to a JSON file. The usual `--clone` flag on
/// `solana-test-validator` does not allow for anything like this.
///
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
        generate_account::js_test_import(&self.save_location())
    }

    /// Default implementation performs no modification
    fn modify(&self, deserialized: Self::T) -> Self::T {
        deserialized
    }

    fn fetch_and_modify_data(&self, client: &RpcClient) -> anyhow::Result<(Account, Self::T)> {
        let address = self.address();
        let info = client
            .get_account(&address)?;
        let deserialized = Self::T::try_deserialize(
            &mut info.data.as_slice())?;
        Ok((info, self.modify(deserialized)))
    }

    fn write_to_validator_json_file(&self, client: &RpcClient) -> anyhow::Result<()> {
        let (info, modified) = self.fetch_and_modify_data(client)?;
        generate_account::write_to_validator_json_file(
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
