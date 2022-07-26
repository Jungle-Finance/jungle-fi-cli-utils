/// A set of instructions can be processed in many ways.
/// The most common way is to sign, serialize, and publish the transaction.
/// Other useful ways to process a transaction include:
/// - Offline signing (sign, serialize, don't send)
/// - Serializing only (so that it can be signed remotely)
/// - Serializing into a multisig transaction proposal
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_client::client_error::ClientErrorKind;
use anchor_client::solana_client::rpc_config::RpcSendTransactionConfig;
use anchor_client::solana_client::rpc_request::{RpcError, RpcRequest, RpcResponseErrorData};
use anchor_client::solana_sdk::bs58;
use anchor_client::solana_sdk::signature::Signature;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::solana_sdk::transaction::Transaction;
use anyhow::Result;
use serde_json::json;
use solana_program::hash::Hash;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;


/// Returns Base58 serialized, signed transaction.
pub fn sign_transaction(
    ixs: Vec<Instruction>,
    fee_payer: &Pubkey,
    signers: &Vec<Box<dyn Signer>>,
    recent_blockhash: Hash,
) -> Result<String> {
    // Send the transaction
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&fee_payer), // payer
        signers,
        recent_blockhash,
    );
    let serialized = bincode::serialize(&tx)?;
    Ok(bs58::encode(&serialized).into_string())
}

/// Serialize the transaction, but do not sign.
/// This function bakes in a fee-payer, but not a blockhash.
pub fn serialize_transaction(
    ixs: Vec<Instruction>,
    fee_payer: &Pubkey,
) -> String {
    let tx = Transaction::new_with_payer(
        &ixs,
        Some(&fee_payer), // payer
    );
    bs58::encode(tx.message.serialize()).into_string()
}

/// If a transaction has already been signed and serialized,
/// this performs the necessary non-standard client RPC call to publish it.
pub fn send_raw_signed_transaction(
    raw_tx_b64: String,
    client: &RpcClient,
) -> Result<Signature> {
    let config = RpcSendTransactionConfig {
        preflight_commitment: Some(
            client.commitment().commitment,
        ),
        ..RpcSendTransactionConfig::default()
    };
    let signature = client.send(
        RpcRequest::SendTransaction,
        json!([raw_tx_b64, config])
    )?;
    Ok(signature)
}

/// Sign and send, with CLI prints
pub fn send_transaction(
    ixs: Vec<Instruction>,
    fee_payer: &Pubkey,
    signers: &Vec<Box<dyn Signer>>,
    client: &RpcClient,
) -> Result<Signature> {
    // Send the transaction
    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&fee_payer), // payer
        signers,
        recent_blockhash,
    );
    let signature = client.send_transaction(&tx);
    match signature {
        Ok(signature) => {
            println!("Success: {}", signature);
            Ok(signature)
        },
        Err(e) => {
            println!("Failure: {}", tx.signatures.first().map_or(
                "<no-signature>".to_string(),
                |s| s.to_string(),
            ));
            let e = process_client_err(e);
            Err(e.into())
        },
    }
}

/// Only used in this module's [send_transaction], just too nested to put directly in.
fn process_client_err(
    err: anchor_client::solana_client::client_error::ClientError
) -> anchor_client::solana_client::client_error::ClientError {
    if let ClientErrorKind::RpcError(err) = &err.kind {
        match err {
            RpcError::RpcResponseError {
                data, ..
            } => {
                if let RpcResponseErrorData::SendTransactionPreflightFailure(
                    result
                ) = data {
                    if let Some(logs) = &result.logs {
                        logs.iter().for_each(|e| println!("{}", e))
                    }
                }
            },
            _ => println!("{}", err.to_string()),
        }
    } else {
        println!("{}", err.to_string());
    }
    err
}
