use solana_client::rpc_client::RpcClient;
use solana_sdk::program_option::COption;
use solana_sdk::pubkey;
use jungle_fi_localnet_tools::localnet_account::LocalnetAccount;
use solana_sdk::pubkey::Pubkey;
use jungle_fi_localnet_tools::{spl_mint_account, spl_token_account, SplMintAccount, SystemAccount};
use jungle_fi_localnet_tools::test_toml_generator::TestTomlGenerator;

pub fn suite_2() -> TestTomlGenerator {
    TestTomlGenerator {
        save_directory: "./tests/suite-2".to_string(),
        test_file_glob: Some("./tests/suite-2/test.ts".to_string()),
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
    let usdc = LocalnetAccount::new_from_clone::<SplMintAccount, _>(
        &pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        &RpcClient::new("https://api.mainnet-beta.solana.com".to_string()),
        "usdc_mint.json".to_string(),
        Some(|mint: SplMintAccount| {
            let mut mint = mint.clone();
            mint.mint_authority = COption::Some(test_user.address.clone());
            SplMintAccount::from_spl_mint(mint)
        })
    ).unwrap();
    vec![
        test_user,
        test_mint,
        usdc,
        test_token_account,
    ]
}
