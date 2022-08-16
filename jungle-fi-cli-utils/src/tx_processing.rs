use anchor_client::solana_client;
use anchor_client::solana_client::rpc_client::RpcClient;
use serde_json::{Map, Value};
use solana_sdk::bs58;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use thiserror::Error;


// pub struct Memo {
//     message: String,
// }
//
// impl TransactionProcessor for Memo {
//     type OnlineArgs = ();
//     type RemainingArgs = ();
//
//     fn get_online_args(&self, _: &RpcClient) -> Result<Self::OnlineArgs, TransactionProcessorError> {
//         Ok(())
//     }
//
//     fn metadata(&self, primary_signer: &Pubkey, _: &Self::OnlineArgs, _: &Self::RemainingArgs) -> Map<String, Value> {
//         let mut map = Map::new();
//         map.insert("message".to_string(), Value::String(self.message.to_string()));
//         map.insert("signer".to_string(), Value::String(primary_signer.to_string()));
//         map
//     }
//
//     fn name(&self, _: &Pubkey, _: &Self::OnlineArgs, _: &Self::RemainingArgs) -> String {
//         format!("memo: {}", self.message)
//     }
//
//     fn calc_remaining_args(&self, _: &Self::OnlineArgs, _: &Pubkey) -> Result<Self::RemainingArgs, TransactionProcessorError> {
//         Ok(())
//     }
//
//     fn create_instructions(&self, primary_signer: &Pubkey, _: Self::OnlineArgs, _: Self::RemainingArgs) -> Result<(Vec<&str>, Vec<Instruction>), TransactionProcessorError> {
//         Ok(
//             (
//                 vec!["memo"],
//                 vec![spl_memo::build_memo(self.message.as_bytes(), &[primary_signer])]
//             )
//         )
//     }
// }

#[derive(Debug, Error)]
pub enum TransactionProcessorError {
    #[error("rpc client error: {0}")]
    ClientError(solana_client::client_error::ClientError),
    #[error("{0}")]
    Other(Box<dyn std::error::Error>),
}

/// If you can calculate values instead of require the user pass them in,
/// then do so in the constructor. If you need to pull cluster data first,
/// then calculate those values in [calc_remaining_args].
pub trait TransactionProcessor {
    type OnlineArgs;
    type RemainingArgs;

    fn get_online_args(&self, _: &RpcClient) -> Result<Self::OnlineArgs, TransactionProcessorError>;

    /// Given everything known about the transaction,
    /// save anything pertinent for user feedback here.
    #[allow(unused)]
    fn metadata(
        &self,
        primary_signer: &Pubkey,
        online_args: &Self::OnlineArgs,
        remaining: &Self::RemainingArgs,
    ) -> Map<String, Value> {
        Map::new()
    }

    /// After fetching online arguments, derive any remaining values
    /// that you need to create instructions.
    fn name(
        &self,
        primary_signer: &Pubkey,
        online_args: &Self::OnlineArgs,
        remaining_args: &Self::RemainingArgs,
    ) -> String;

    /// After fetching online arguments, derive any remaining values
    /// that you need to create instructions.
    fn calc_remaining_args(
        &self,
        online_args: &Self::OnlineArgs,
        primary_signer: &Pubkey,
    ) -> Result<Self::RemainingArgs, TransactionProcessorError>;

    /// Create a vec of instructions paired with names
    fn create_instructions(
        &self,
        primary_signer: &Pubkey,
        online_args: Self::OnlineArgs,
        remaining: Self::RemainingArgs,
    ) -> Result<(Vec<&str>, Vec<Instruction>), TransactionProcessorError>;

    fn process(
        &self,
        mode: Processing<Self::OnlineArgs>,
        extra_signers: &mut Vec<Box<dyn Signer>>,
    ) -> Result<ProcessedTransaction, TransactionProcessorError> {
        match mode {
            Processing::Execute(client, signer) => {
                let primary_signer = signer.pubkey();
                let online_args = self.get_online_args(&client)?;
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                extra_signers.push(signer);
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (_, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let recent_blockhash = client.get_latest_blockhash()
                    .map_err(|e| TransactionProcessorError::ClientError(e))?;
                let tx = Transaction::new_signed_with_payer(
                    &ixs,
                    Some(&primary_signer), // payer
                    extra_signers,
                    recent_blockhash,
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| TransactionProcessorError::ClientError(e))?;
                Ok(ProcessedTransaction::Execution {
                    name,
                    signature: signature.to_string(),
                    metadata,
                })
            }
            Processing::Sign(client, signer) => {
                let primary_signer = signer.pubkey();
                let online_args = self.get_online_args(&client)?;
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                extra_signers.push(signer);
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (_, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let recent_blockhash = client.get_latest_blockhash()
                    .map_err(|e| TransactionProcessorError::ClientError(e))?;
                let tx = Transaction::new_signed_with_payer(
                    &ixs,
                    Some(&primary_signer), // payer
                    extra_signers,
                    recent_blockhash,
                );
                let serialized = bincode::serialize(&tx)
                    .expect("transaction failed to serialize");
                Ok(ProcessedTransaction::SignedSerialized {
                    transaction: bs58::encode(&serialized).into_string(),
                    name,
                    metadata,
                })
            }
            Processing::Serialize(client, primary_signer) => {
                let online_args = self.get_online_args(&client)?;
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (_, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let tx = Transaction::new_with_payer(
                    &ixs,
                    Some(&primary_signer), // payer
                );
                Ok(ProcessedTransaction::UnsignedSerialized {
                    transaction: bs58::encode(tx.message.serialize()).into_string(),
                    name,
                    metadata,
                })
            }
            Processing::Instructions(client, primary_signer) => {
                let online_args = self.get_online_args(&client)?;
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (names, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let ixs = ixs.iter().map(
                    serialize_ix
                ).collect();
                Ok(ProcessedTransaction::InstructionSet {
                    instructions: ixs,
                    instruction_names: names.iter().map(|s| s.to_string()).collect(),
                    name,
                    metadata,
                })
            }
            Processing::OfflineSign(online_args, signer, recent_blockhash) => {
                let primary_signer = signer.pubkey();
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                extra_signers.push(signer);
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (_, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let tx = Transaction::new_signed_with_payer(
                    &ixs,
                    Some(&primary_signer), // payer
                    extra_signers,
                    recent_blockhash,
                );
                let serialized = bincode::serialize(&tx)
                    .expect("transaction failed to serialize");
                Ok(ProcessedTransaction::SignedSerialized {
                    transaction: bs58::encode(serialized).into_string(),
                    name,
                    metadata
                })
            }
            Processing::OfflineSerialize(online_args, primary_signer) => {
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (_, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let tx = Transaction::new_with_payer(
                    &ixs,
                    Some(&primary_signer), // payer
                );
                Ok(ProcessedTransaction::UnsignedSerialized {
                    transaction: bs58::encode(tx.message.serialize()).into_string(),
                    name,
                    metadata,
                })
            }
            Processing::OfflineInstructions(online_args, primary_signer) => {
                let remaining_args = self.calc_remaining_args(
                    &online_args,
                    &primary_signer,
                )?;
                let name = self.name(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let metadata = self.metadata(
                    &primary_signer,
                    &online_args,
                    &remaining_args,
                );
                let (names, ixs) = self.create_instructions(
                    &primary_signer,
                    online_args,
                    remaining_args,
                )?;
                let ixs = ixs.iter().map(
                    serialize_ix
                ).collect();
                Ok(ProcessedTransaction::InstructionSet {
                    instructions: ixs,
                    instruction_names: names.iter().map(|s| s.to_string()).collect(),
                    name,
                    metadata,
                })
            }
        }
    }
}

fn serialize_ix(ix: &Instruction) -> String {
    bs58::encode(
        bincode::serialize(ix).expect("instruction failed to serialize")
    ).into_string()
}

/// Offline variants require passing in some [T] which would
/// normally come from querying the cluster.
pub enum Processing<T> {
    /// Requires network traffic
    Execute(RpcClient, Box<dyn Signer>),
    Sign(RpcClient, Box<dyn Signer>),
    Serialize(RpcClient, Pubkey), // client, signer
    Instructions(RpcClient, Pubkey), // client, multisig_signer
    /// Does not require any network traffic
    OfflineSign(T, Box<dyn Signer>, Hash),
    OfflineSerialize(T, Pubkey),
    OfflineInstructions(T, Pubkey),
}

impl<T> Processing<T> {
    /// Calculates the multisig signer given a multisig account and
    /// the multisig program ID.
    pub fn propose(
        client: RpcClient,
        program_id: &Pubkey,
        multisig: &Pubkey
    ) -> Self {
        Processing::Instructions(
            client,
            Pubkey::find_program_address(&[multisig.as_ref()], program_id).0,
        )
    }

    pub fn propose_offline(
        online_args: T,
        program_id: &Pubkey,
        multisig: &Pubkey
    ) -> Self {
        Processing::OfflineInstructions(
            online_args,
            Pubkey::find_program_address(&[multisig.as_ref()], program_id).0,
        )
    }
}

pub enum ProcessedTransaction {
    Execution {
        signature: String,
        name: String,
        metadata: Map<String, Value>,
    },
    SignedSerialized {
        transaction: String,
        name: String,
        metadata: Map<String, Value>,
    },
    UnsignedSerialized {
        transaction: String,
        name: String,
        metadata: Map<String, Value>,
    },
    InstructionSet {
        instructions: Vec<String>,
        instruction_names: Vec<String>,
        name: String,
        metadata: Map<String, Value>,
    },
}
