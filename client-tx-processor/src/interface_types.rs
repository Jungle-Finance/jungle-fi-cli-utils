use anchor_client::solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signer;
use anchor_client::anchor_lang::prelude::Pubkey;
use anchor_client::anchor_lang::solana_program::hash::Hash;
use serde_json::{Map, Value};
use anchor_client::solana_client::rpc_response::{RpcResponseContext, RpcSimulateTransactionResult};

/// Offline variants require passing in some [T] which would
/// normally come from querying the cluster.
/// [Offline*] variants do not require network traffic, but [online_args: T] must be created
/// by other means.
pub enum Processing<T> {
    /// Sign, serialize, and send the transaction for execution on the cluster.
    Execute(RpcClient, Box<dyn Signer>),
    /// Sign, serialize, and simulate the transaction.
    Simulate(RpcClient, Box<dyn Signer>),
    /// Sign and serialize the transaction. Useful to hand to third parties
    /// for additional requires signatures before publishing the transaction on-chain.
    Sign(RpcClient, Box<dyn Signer>),
    /// No signatures applied, simply the Transaction Message serialized.
    Serialize(RpcClient, Pubkey), // client, signer
    /// Output the transaction instructions in Base58 encoding. This allows one to compose
    /// multisig proposals.
    Instructions(RpcClient, Pubkey), // client, multisig_signer
    /// Similar to [Processing<T>::Sign], except prerequisite data must be created offline.
    OfflineSign(T, Box<dyn Signer>, Hash),
    /// Similar to [Processing<T>::Serialize], except prerequisite data must be created offline.
    OfflineSerialize(T, Pubkey),
    /// Similar to [Processing<T>::Instructions], except prerequisite data must be created offline.
    OfflineInstructions(T, Pubkey),
}

/// The return type for [TransactionProcessor::process].
pub enum ProcessedTransaction {
    /// Pertinent information after a transaction has been successfully executed.
    Execution {
        signature: String,
        name: String,
        metadata: Map<String, Value>,
    },
    /// Pertinent information after a transaction has been successfully simulated.
    Simulation {
        name: String,
        metadata: Map<String, Value>,
        simulation_result: RpcSimulateTransactionResult,
        simulation_context: RpcResponseContext,
    },
    /// The signed/serialized transaction, plus related pertinent information.
    SignedSerialized {
        transaction: String,
        name: String,
        metadata: Map<String, Value>,
    },
    /// The unsigned/serialized transaction, plus related pertinent information.
    UnsignedSerialized {
        transaction: String,
        name: String,
        metadata: Map<String, Value>,
    },
    /// A list of instructions in Base58 encoding.
    InstructionSet {
        instructions: Vec<String>,
        instruction_names: Vec<String>,
        name: String,
        metadata: Map<String, Value>,
    },
}
