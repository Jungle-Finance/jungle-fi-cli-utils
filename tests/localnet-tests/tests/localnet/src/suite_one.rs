use jungle_fi_localnet_tools::localnet_account::LocalnetAccount;
use solana_sdk::pubkey::Pubkey;
use jungle_fi_localnet_tools::{spl_mint_account, spl_token_account, SystemAccount};
use jungle_fi_localnet_tools::test_toml_generator::TestTomlGenerator;

pub fn suite_1() -> TestTomlGenerator {
    TestTomlGenerator {
        save_directory: "suite-1".to_string(),
        test_file_glob: Some("./tests/suite-1/test.ts".to_string()),
        accounts: accounts(),
        ..Default::default()
    }
}

pub fn accounts() -> Vec<LocalnetAccount> {
    let test_user = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user.json".to_string(),
        SystemAccount,
    );
    let test_mint = LocalnetAccount::new(
        Pubkey::new_unique(),
        "mint.json".to_string(),
        spl_mint_account(&test_user.address, 0, 9),
    );
    let test_token_account = LocalnetAccount::new(
        Pubkey::new_unique(),
        "test_user_token_act.json".to_string(),
        spl_token_account(
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
