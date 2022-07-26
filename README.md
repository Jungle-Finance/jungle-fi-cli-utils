## Jungle Finance Solana CLI Utilities

This library contains solutions to problems that commonly occur
when creating anything command-line related for Solana.

There are QoL functions in here to assist with:
- Clap command-line input parsing
- Functions that conform to how the official Solana CLI suite handles signer and cluster configuration.
- Account data parsing, generation, (de-)serialization
- Transaction execution, signing, and serialization

#### Question: Why is Solana clap-v3 utils and Project Serum Multisig in here?
Because we want to use them as dependencies, but there are version conflicts
with the officially released Anchor crates.
The cargo-culted clap-v3 crate will get phased out when Anchor updates to use >=1.11.0,
and the multisig instance will get phased out when its upstream Cargo.toml bumps to 0.25.0.
