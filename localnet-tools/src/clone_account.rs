use anchor_client::anchor_lang::{AccountDeserialize, AccountSerialize};
use solana_program::pubkey::Pubkey;
use anchor_client::solana_client::rpc_client::RpcClient;
use solana_sdk::account::{Account, AccountSharedData, ReadableAccount};
use crate::generate_account;

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

    // Can add this data type directly to a [TestValidatorGenesis] accounts to load.
    fn account_shared_data(&self, client: &RpcClient) -> anyhow::Result<(Pubkey, AccountSharedData)> {
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