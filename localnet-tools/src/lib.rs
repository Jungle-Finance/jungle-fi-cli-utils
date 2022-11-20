/// A variety of QoL functions and tooling to do extensive
/// localnet setup and testing.
use std::io::Write;
use anchor_client::anchor_lang;
use solana_program::pubkey::Pubkey;
use anchor_client::anchor_lang::prelude::System;
use anchor_client::anchor_lang::Id;

mod wrapped_spl_types;
pub mod test_toml_generator;
pub mod localnet_account;
pub mod trait_based;
pub mod idl;
pub mod test_validator;
pub mod cli;

pub use localnet_account::LocalnetAccount;
pub use test_toml_generator::TestTomlGenerator;
pub use wrapped_spl_types::{spl_mint_account, SplMintAccount, spl_token_account, SplTokenAccount};

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
