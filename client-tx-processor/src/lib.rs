mod error;
mod interface_types;
/// Define a struct representing a transaction schema.
/// Implementing [TransactionProcessor] allows for a number of
/// approaches to processing the transaction, from the most common
/// case of signing and sending, to more niche cases of printing instruction
/// data to use as a multisig proposal.
///
/// This can be used in both CLI or servers.
/// This is only an advisable approach when you have some standardized transaction schemas,
/// and you need multiple forms of transaction processing. Otherwise, this is all overkill.
use anchor_client::solana_client::rpc_client::RpcClient;
use serde_json::{Map, Value};
use solana_sdk::bs58;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

pub use error::TransactionProcessorError;
pub use interface_types::{ProcessedTransaction, Processing};
use crate::error::maybe_print_preflight_simulation_logs;


/// If you can calculate values instead of require the user pass them in,
/// then do so in the constructor. If you need to pull cluster data first,
/// then calculate those values in [calc_remaining_args].
pub trait TransactionProcessor {
    /// Anything you need to fetch online first before instantiating the transaction.
    type OnlineArgs;
    /// Anything needed that can be further derived after fetching [OnlineArgs].
    type RemainingArgs;

    /// Sometimes we do not have to pass in prerequisite data, and can much more reliably
    /// acquire accurate values for such data by simply querying blockchain state.
    /// This function is a place to do that.
    #[allow(unused)]
    fn get_online_args(&self, client: &RpcClient) -> Result<Self::OnlineArgs, TransactionProcessorError>;

    /// Given everything known about the transaction,
    /// save anything pertinent for user feedback here.
    /// e.g. Sometimes an account is created during a transaction execution,
    /// and the user needs to know the address of that new account.
    /// This provides a simple [serde_json] based way to dump any arbitrary data
    /// associated with the transaction.
    #[allow(unused)]
    fn metadata(
        &self,
        primary_signer: &Pubkey,
        online_args: &Self::OnlineArgs,
        remaining: &Self::RemainingArgs,
    ) -> Map<String, Value> {
        Map::new()
    }

    /// After fetching all necessary values to create the transaction,
    /// output a name for the transaction.
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

    /// Create a vec of instructions paired with names.
    /// Creates a tuple of two vectors:
    /// - [Vec<Instruction>] represents an ordered list of instructions
    /// to add to the transaction.
    /// - [Vec<&str>] represents the names for each instruction, where the corresponding
    /// indices match across both this vec and the [Vec<Instruction>].
    fn create_instructions(
        &self,
        primary_signer: &Pubkey,
        online_args: Self::OnlineArgs,
        remaining: Self::RemainingArgs,
    ) -> Result<(Vec<&str>, Vec<Instruction>), TransactionProcessorError>;

    /// Runs the transaction processing, according to the given mode of processing.
    /// This
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
                    .map_err(|e| {
                        let e = maybe_print_preflight_simulation_logs(e);
                        TransactionProcessorError::ClientError(e)
                    })?;
                Ok(ProcessedTransaction::Execution {
                    name,
                    signature: signature.to_string(),
                    metadata,
                })
            }
            Processing::Simulate(client, signer) => {
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
                let response = client.simulate_transaction(&tx)
                    .map_err(|e| {
                        let e = maybe_print_preflight_simulation_logs(e);
                        TransactionProcessorError::ClientError(e)
                    })?;
                let result = response.value;
                let context = response.context;
                Ok(ProcessedTransaction::Simulation {
                    name,
                    metadata,
                    simulation_result: result,
                    simulation_context: context,

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

/// Base-58 encode an [Instruction] from the Solana SDK.
fn serialize_ix(ix: &Instruction) -> String {
    bs58::encode(
        bincode::serialize(ix).expect("instruction failed to serialize")
    ).into_string()
}


#[cfg(test)]
mod tests {
    use solana_sdk::hash::Hash;
    use solana_sdk::signature::Keypair;
    use super::*;

    /// Simple memo transaction
    pub struct Memo {
        message: String,
    }

    impl TransactionProcessor for Memo {
        type OnlineArgs = ();
        type RemainingArgs = ();

        fn get_online_args(&self, _: &RpcClient) -> Result<Self::OnlineArgs, TransactionProcessorError> {
            Ok(())
        }

        fn metadata(&self, primary_signer: &Pubkey, _: &Self::OnlineArgs, _: &Self::RemainingArgs) -> Map<String, Value> {
            let mut map = Map::new();
            map.insert("message".to_string(), Value::String(self.message.to_string()));
            map.insert("signer".to_string(), Value::String(primary_signer.to_string()));
            map
        }

        fn name(&self, _: &Pubkey, _: &Self::OnlineArgs, _: &Self::RemainingArgs) -> String {
            format!("memo: {}", self.message)
        }

        fn calc_remaining_args(&self, _: &Self::OnlineArgs, _: &Pubkey) -> Result<Self::RemainingArgs, TransactionProcessorError> {
            Ok(())
        }

        fn create_instructions(&self, primary_signer: &Pubkey, _: Self::OnlineArgs, _: Self::RemainingArgs) -> Result<(Vec<&str>, Vec<Instruction>), TransactionProcessorError> {
            Ok(
                (
                    vec!["memo"],
                    vec![spl_memo::build_memo(self.message.as_bytes(), &[primary_signer])]
                )
            )
        }
    }

    #[test]
    fn execution() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let client = RpcClient::new_mock("succeeds");
        let response = memo_tx.process(
            Processing::Execute(client, Box::new(signer)),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::Execution {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn simulation() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let client = RpcClient::new_mock("succeeds");
        let response = memo_tx.process(
            Processing::Simulate(client, Box::new(signer)),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::Simulation {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn sign() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let client = RpcClient::new_mock("succeeds");
        let response = memo_tx.process(
            Processing::Sign(client, Box::new(signer)),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::SignedSerialized {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn serialize() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let client = RpcClient::new_mock("succeeds");
        let response = memo_tx.process(
            Processing::Serialize(client, signer.pubkey()),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::UnsignedSerialized {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn instructions() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let client = RpcClient::new_mock("succeeds");
        let response = memo_tx.process(
            Processing::Instructions(client, signer.pubkey()),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::InstructionSet {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn offline_sign() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let response = memo_tx.process(
            Processing::OfflineSign((), Box::new(signer), Hash::new_unique()),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::SignedSerialized {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn offline_serialize() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let response = memo_tx.process(
            Processing::OfflineSerialize((), signer.pubkey()),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::UnsignedSerialized {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }

    #[test]
    fn offline_instructions() {
        let memo_tx = Memo {
            message: "Foobar".to_string()
        };

        let signer = Keypair::new();
        let response = memo_tx.process(
            Processing::OfflineInstructions((), signer.pubkey()),
            &mut vec![],
        ).unwrap();
        if let ProcessedTransaction::InstructionSet {
            name,
            ..
        } = response {
            assert_eq!(name, "memo: Foobar".to_string());
        } else {
            panic!("wrong processing");
        }
    }
}