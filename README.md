## Jungle Finance Solana CLI Utilities

This library contains solutions to problems that commonly occur
when creating anything command-line related for Solana.

They are broken down into the following crates:
1. `jungle-fi-cli-tools` -- A collection of tools for adopting the same interface for keypairs
and urls as the official Solana CLI's, as well as a few other QoL CLI related stuff.
2. `genesys-go-rpc-auth` -- Simple implementation of a client request to acquire a bearer token
with the authenticated GenesysGo RPC service.
3. `solana-rpc-client-headers` -- Allows for attaching custom headers to `RPCClient` requests. This is useful for
example when one needs to add an authentication header to interact with their RPC provider.
4. `threadsafe-signer` -- Wrapper struct for any `T: Signer` to give it `Clone + Send + Sync` for thread-safety.
5. `jungle-fi-localnet-tools` -- A library for writing binaries that can generate complicated localnet setups,
with the ability to modularize test suites across many auto-generated `Test.toml` files. Each test suite
can have its own test validator configurations, accounts, and programs, etc. Other QoL includes modifying cloned accounts,
and an auto-generated JS file to import pubkeys for all the pre-loaded test accounts into your `.ts` tests.
6. `solana-client-tx-processor` -- A library that allows defining Transaction schemas with minimal constructor interfaces
and a large number of possible means of processing the transaction. Means of processing include: sending,
signing, serializing, simulating, or printing out the instructions as serialized data, as well as offline variants
of these where applicable.

#### Question: Why is Solana clap-v3 utils in here?
Because we want to use it as a dependency, but there are version conflicts
with the officially released Anchor crates.
The cargo-culted clap-v3 crate will get phased out when the latest Anchor release supports Solana >=1.11.0,
