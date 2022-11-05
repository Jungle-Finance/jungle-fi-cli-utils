use anchor_client::solana_client;
use anchor_client::solana_client::client_error::ClientErrorKind;
use anchor_client::solana_client::rpc_request::{RpcError, RpcResponseErrorData};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionProcessorError {
    #[error("rpc client error: {0}")]
    ClientError(solana_client::client_error::ClientError),
    #[error("{0}")]
    Other(Box<dyn std::error::Error>),
}

// TODO Use this in the above to improve error handling
pub fn process_client_err(
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
