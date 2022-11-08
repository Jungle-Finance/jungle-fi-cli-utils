## JFI Solana Localnet Tools

This crate solves a few problems with using `--account` on `solana-test-validator`:
- Serializing account data into strings ready for JSON objects.
- Staying as "close-to-the-source" as possible when it comes to contract code.
- Managing a potentially large amount of accounts and/or data value interdependencies (e.g. PDA values).
- Filling in the serialization gaps left over in certain structs in `anchor_spl::token`.


## TODO
Ditch the Bash script "cmd arg" way of doing this.
Create Test.toml files!
You basically need a struct or something, some way to group together a bunch of:
1. Programs that should be deployed via `[test.genesis]`
2. JSON account files created by running a Rust binary, and included with `[test.validator.account]`.
3. Anything to be cloned live from mainnet.
4. A specification of the string that represents test file target.
5. A list of any validator arguments.
6. A specification of where the `Test.toml` file should be written to.
7. A simple, standardized way to handle the JSON file organization relative to where the tests and Test.toml files live
8. Basically create one of those _TestToml structs, and dump the toml to a file.

tests/
tests/suite-1/
tests/suite-1/Test.toml
tests/suite-1/suite-1.0.ts
tests/suite-1/suite-1.1.ts
tests/suite-2/
tests/suite-2/Test.toml
tests/suite-2/suite-2.0.ts
tests/suite-2/suite-2.1.ts
tests/accounts/
tests/accounts/act1.json
tests/accounts/act2.json
tests/accounts/foo/
tests/accounts/foo/act1.json
tests/accounts/foo/act2.json
tests/genesis/
tests/genesis/third_party_program.so

So there are three addresses:
Two addresses for every JSON file:
1. The save location (relative to where you execute the Rust binary that generates the JSON and TOML files)
2. The address that goes into the Test.toml file (relative to the Test.toml file)
Then there's a third address for the Test.toml file
  (like #1, relative to where you execute the Rust binary that generates the JSON and TOML files).

We can call them:
- save_location
- relative_from_test_toml

You need `anchor_cli::_TestToml`. Then `toml::to_str(test_toml)` and write to file.

How to get to that `_TestToml` type.
A bunch of `anchor_cli::config::AccountEntry` (address, filepath) which goes into a `anchor_cli::config::_Validator`, which in turn goes into a `anchor_cli::config::_TestValidator`.
The `_Validator` also takes all test validator options as well.
A bunch of `anchor_cli::config::GenesisEntry` (address, filepath) which goes into `anchor_cli::config::_TestValidator`.
`_TestValidator` also takes the startup and shutdown wait times.
Using the `_TestValidator`, any `extends` to add, and `scripts`, you then can make a `_TestToml` that you can write to a file.

The `scripts` field takes `Vec` of `ScriptsConfig` which are really just `BTreeMap<String, String>`.
Every `scripts` should contain only one key-value pair:
- key: `"test"`
- value: `format!("yarn run ts-mocha -p ./tsconfig.json -t 1000000" {}", test_file_glob)`

In total, I need to know for every test suite:
0. A Save Location for the `Test.toml` file.
1. A (potentially empty) list of things I can routinely turn into both (whether cloned or generated):
   a. `AccountEntry`, whose filepaths are relative to where the `Test.toml` file will live.
   b. JSON files, whose filepaths are going to obey (a) given where I run the Rust binary.
2. A (potentially empty) list of programID and filepath pairings (I will always do `"../genesis/prog.so"`).
3. (OPTIONAL) A list of validator options (pass this in as an owned mutable `_Validator`).
4. (OPTIONAL) A string for the `test_file_glob` in the scripts section.
5. (OPTIONAL) Vec of `extends` strings (relative to TOML file's eventual location).


Maybe the TestTomlGenerator should store a "path prefix" for:
1. The directory where all the JSON files will be saved. (default tests/accounts)
2. The relative path to (1) from where the `Test.toml` will live. (default ../accounts)

#### CLI Wishlist Thing
A CLI that does better localnet stuff. I want to be able to:
1. Run a single test suite (a single Test.toml, or any subset of them).
2. Run a localnet configured from a Test.toml.
3. Execute the whole JSON dump + JS file dump + Test.toml file dump.