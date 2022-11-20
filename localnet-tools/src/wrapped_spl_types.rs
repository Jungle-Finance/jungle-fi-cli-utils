/// These are a more complete implementation of the Anchor (de-)serialization layer on
/// SPL Token types. We can delegate to the underlying implementation for deserialization,
/// but we need to add a non-default implementation of [anchor_lang::AccountSerialize],
/// as that method's current default implementation writes nothing to the byte-buffer.
///
/// To generate or clone an SPL-token account, use these types as your
/// [GeneratedAccount::T] or [ClonedAccount::T].
use std::io::Write;
use std::ops::Deref;
use anchor_client::anchor_lang::{AccountDeserialize, AccountSerialize, Owner};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::program_option::COption;

/// [anchor_spl::token::Mint] omits a serialization implementation,
/// preventing it from being useful in a number of use-cases.
/// [MintWrapper] solves this by re-implementing the relevant Anchor traits,
/// delegating implementation to the `anchor_spl` type where possible.
pub struct SplMintAccount(anchor_spl::token::Mint);

impl SplMintAccount {
    pub const LEN: usize = spl_token::state::Mint::LEN;

    /// Preferred factory function to convert into this data type.
    pub fn from_mint(mint: anchor_spl::token::Mint) -> Self {
        Self(mint)
    }
}

/// Anchor uses this trait to enforce proper program ownership during account deserialization.
impl Owner for SplMintAccount {
    fn owner() -> Pubkey {
        anchor_spl::token::Mint::owner()
    }
}

impl AccountDeserialize for SplMintAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_client::anchor_lang::Result<Self> {
        let mint = anchor_spl::token::Mint::try_deserialize_unchecked(buf)?;
        Ok(SplMintAccount(mint))
    }
}

/// This is the primary reason this entire codebase exists, otherwise we could simply
/// use the [anchor-spl] crate.
impl AccountSerialize for SplMintAccount {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> anchor_client::anchor_lang::Result<()> {
        // these are the only four lines that matter
        let mint = &self.0;
        let mut serialized = vec!(0; 82);  // len found in `mint_act.pack_to_slice`
        mint.pack_into_slice(&mut serialized);
        _writer.write_all(&serialized)?;
        Ok(())
    }
}

/// Unpacks the wrapper class into its wrapped type.
impl Deref for SplMintAccount {
    type Target = spl_token::state::Mint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// [anchor_spl::token::TokenAccount] omits a serialization implementation,
/// preventing it from being useful in a number of use-cases.
/// [TokenAccountWrapper] solves this by re-implementing the relevant Anchor traits,
/// delegating implementation to the `anchor_spl` type where possible.
pub struct SplTokenAccount(anchor_spl::token::TokenAccount);

impl SplTokenAccount {
    pub const LEN: usize = spl_token::state::Account::LEN;

    /// Preferred factory function to convert into this data type.
    pub fn from_token_account(token_account: anchor_spl::token::TokenAccount) -> Self {
        Self(token_account)
    }
}

/// Anchor uses this trait to enforce proper program ownership during account deserialization.
impl Owner for SplTokenAccount {
    fn owner() -> Pubkey {
        anchor_spl::token::TokenAccount::owner()
    }
}

impl AccountDeserialize for SplTokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_client::anchor_lang::Result<Self> {
        let token_act = anchor_spl::token::TokenAccount::try_deserialize_unchecked(buf)?;
        Ok(SplTokenAccount(token_act))
    }
}

/// This is again the primary reason this entire codebase exists, otherwise we could simply
/// use the [anchor-spl] crate.
impl AccountSerialize for SplTokenAccount {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> anchor_client::anchor_lang::Result<()> {
        // these are the only four lines that matter
        let token_account = &self.0;
        let mut serialized = vec!(0; 165);  // len found in `token_account.pack_to_slice`
        token_account.pack_into_slice(&mut serialized);
        _writer.write_all(&serialized)?;
        Ok(())
    }
}

/// Unpacks the wrapper class into its wrapped type.
impl Deref for SplTokenAccount {
    type Target = spl_token::state::Account;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Convenience function, basically a constructor with some opinionated defaults.
/// See source code below for which parameters are chosen for the user.
pub fn spl_mint_account(
    authority: &Pubkey,
    supply: u64,
    decimals: u8,
) -> anchor_spl::token::Mint {
    let mint_act = spl_token::state::Mint {
        mint_authority: COption::Some(*authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::Some(*authority),
    };
    // Since [anchor_spl::Mint] has no public constructor other than deserialization,
    // We have to do it this way if we want to wield an Anchor-compatible object
    // instead of the vanilla [spl_token::state::Account].
    let mut serialized = vec!(0; 82);  // len found in `mint_act.pack_to_slice`
    mint_act.pack_into_slice(& mut serialized);
    anchor_spl::token::Mint::try_deserialize(&mut serialized.as_slice()).unwrap()
}

/// Convenience function, basically a constructor with some opinionated defaults.
/// See source code below for which parameters are chosen for the user.
pub fn spl_token_account(
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> anchor_spl::token::TokenAccount {
    let token_act = spl_token::state::Account {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: spl_token::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::Some(*owner),
    };
    // Since [anchor_spl::TokenAccount] has no public constructor other than deserialization,
    // We have to do it this way if we want to wield an Anchor-compatible object
    // instead of the vanilla `spl_token::state::Account`.
    let mut serialized = vec!(0; 165);
    token_act.pack_into_slice(& mut serialized);
    anchor_spl::token::TokenAccount::try_deserialize(&mut serialized.as_slice()).unwrap()
}