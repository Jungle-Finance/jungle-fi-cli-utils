## JFI Solana Localnet Tools

This crate solves a few problems with using `--account` on `solana-test-validator`:
- Serializing account data into strings ready for JSON objects.
- Staying as "close-to-the-source" as possible when it comes to contract code.
- Managing a potentially large amount of accounts and/or data value interdependencies (e.g. PDA values).
- Filling in the serialization gaps left over in certain structs in `anchor_spl::token`.