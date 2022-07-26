/// Implements SPL Token types, mostly delegating to the underlying
/// Anchor SPL implementation, but with an added implementation of
/// `anchor_lang::AccountSerialize`. The vanilla Anchor SPL types
/// use the default implementation of `AccountSerialize::try_serialize`,
/// which does not write anything to the `T: Write` passed into it.
use std::io::Write;
use std::ops::Deref;
use anchor_client::anchor_lang::{AccountDeserialize, AccountSerialize, Owner};
use anchor_spl::token::{Mint, TokenAccount};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::program_option::COption;
use solana_sdk::pubkey;

pub const SPL_TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// The Mint account type for SPL Token Program.
pub struct MintWrapper(Mint);

impl MintWrapper {
    pub const LEN: usize = spl_token::state::Mint::LEN;

    pub fn from_mint(mint: anchor_spl::token::Mint) -> Self {
        Self(mint)
    }
}

impl Owner for MintWrapper {
    fn owner() -> Pubkey {
        Mint::owner()
    }
}

impl AccountDeserialize for MintWrapper {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_client::anchor_lang::Result<Self> {
        let mint = Mint::try_deserialize_unchecked(buf)?;
        Ok(MintWrapper(mint))
    }
}

impl AccountSerialize for MintWrapper {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> anchor_client::anchor_lang::Result<()> {
        // these are the only four lines that matter
        let mint = &self.0;
        let mut serialized = vec!(0; 82);  // len found in `mint_act.pack_to_slice`
        mint.pack_into_slice(&mut serialized);
        _writer.write_all(&serialized)?;
        Ok(())
    }
}

impl Deref for MintWrapper {
    type Target = spl_token::state::Mint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The TokenAccount account type for SPL Token Program.
pub struct TokenAccountWrapper(TokenAccount);

impl TokenAccountWrapper {
    pub const LEN: usize = spl_token::state::Account::LEN;

    pub fn from_token_account(token_account: anchor_spl::token::TokenAccount) -> Self {
        Self(token_account)
    }
}

impl Owner for TokenAccountWrapper {
    fn owner() -> Pubkey {
        TokenAccount::owner()
    }
}

impl AccountDeserialize for TokenAccountWrapper {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_client::anchor_lang::Result<Self> {
        let token_act = TokenAccount::try_deserialize_unchecked(buf)?;
        Ok(TokenAccountWrapper(token_act))
    }
}

impl AccountSerialize for TokenAccountWrapper {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> anchor_client::anchor_lang::Result<()> {
        // these are the only four lines that matter
        let token_account = &self.0;
        let mut serialized = vec!(0; 165);  // len found in `token_account.pack_to_slice`
        token_account.pack_into_slice(&mut serialized);
        _writer.write_all(&serialized)?;
        Ok(())
    }
}

impl Deref for TokenAccountWrapper {
    type Target = spl_token::state::Account;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Convenience function, basically a constructor with some opinionated defaults.
pub fn arbitrary_mint_account(
    authority: &Pubkey,
    supply: u64,
    decimals: u8,
) -> Mint {
    let mint_act = spl_token::state::Mint {
        mint_authority: COption::Some(*authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::Some(*authority),
    };
    let mut serialized = vec!(0; 82);  // len found in `mint_act.pack_to_slice`
    mint_act.pack_into_slice(& mut serialized);
    Mint::try_deserialize(&mut serialized.as_slice()).unwrap()
}

/// Convenience function, basically a constructor with some opinionated defaults.
pub fn arbitrary_token_account(
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> TokenAccount {
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
    // Since TokenAccount has no public constructor other than deserialization,
    // We have to do it this way if we want to wield an Anchor-compatible object
    // instead of the vanilla `spl_token::state::Acccount`.
    let mut serialized = vec!(0; 165);
    token_act.pack_into_slice(& mut serialized);
    TokenAccount::try_deserialize(&mut serialized.as_slice()).unwrap()
}
