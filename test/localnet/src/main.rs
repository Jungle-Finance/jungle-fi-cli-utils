use solana_sdk::pubkey::Pubkey;
use jungle_fi_localnet_tools::localnet_account::LocalnetAccount;
use jungle_fi_localnet_tools::{arbitrary_mint_account, arbitrary_token_account, SystemAccount};
use jungle_fi_localnet_tools::test_toml::TestTomlGenerator;

fn accounts() -> Vec<LocalnetAccount> {
    let test_user = LocalnetAccount::new(
            Pubkey::new_unique(),
            "test_user.json".to_string(),
            SystemAccount,
        );
    let test_mint = LocalnetAccount::new(
        Pubkey::new_unique(),
        "mint.json".to_string(),
        arbitrary_mint_account(&test_user.address, 0, 9),
    );
    let test_token_account = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user_token_act.json".to_string(),
        arbitrary_token_account(
            &test_mint.address,
            &test_user.address,
            0
        )
    );
    vec![
        test_user,
        test_mint,
        test_token_account,
    ]
}

fn main() -> anyhow::Result<()> {

    let toml = TestTomlGenerator {
        save_directory: "tests/suite-1".to_string(),
        accounts: accounts(),
        ..Default::default()
    };
    toml.build()?;
    Ok(())
}
