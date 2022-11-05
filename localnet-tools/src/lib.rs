/// A variety of QoL functions and tooling to do extensive
/// localnet setup and testing.
use std::io::Write;
use anchor_client::anchor_lang;
use solana_program::pubkey::Pubkey;
use solana_program::clock::Epoch;
use solana_account_decoder::{UiAccount, UiAccountData, UiAccountEncoding};
use anchor_client::anchor_lang::prelude::System;
use anchor_client::anchor_lang::Id;
use solana_sdk::bs58;

mod wrapped_spl_types;
mod generate_account;
mod clone_account;
pub mod anchor_test_log_workaround;

pub use generate_account::GeneratedAccount;
pub use clone_account::ClonedAccount;
pub use wrapped_spl_types::{arbitrary_mint_account, MintWrapper, arbitrary_token_account, TokenAccountWrapper};

/// Creates account info struct of the correct type
/// expected by `solana-test-validator --account`.
/// Handles some tricky structs for the data field.
pub fn ui_account(
    lamports: u64,
    data: &[u8],
    owner: &Pubkey,
    executable: bool,
    rent_epoch: Epoch,
) -> UiAccount {
    UiAccount {
        lamports,
        data: UiAccountData::Binary(
            bs58::encode(data).into_string(),
            UiAccountEncoding::Base58
        ),
        owner: owner.to_string(),
        executable,
        rent_epoch,
    }
}

/// Use this struct as type T for any [GeneratedAccount] or [ClonedAccount]
/// owned by `SystemProgram` (e.g. typical user accounts).
pub struct SystemAccount;

impl SystemAccount {
    pub const LEN: usize = 0;
}

impl anchor_lang::Owner for SystemAccount {
    fn owner() -> Pubkey {
        System::id()
    }
}

impl anchor_lang::AccountDeserialize for SystemAccount {
    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Ok(SystemAccount)
    }
}

impl anchor_lang::AccountSerialize for SystemAccount {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> anchor_lang::Result<()> {
        Ok(())
    }
}
