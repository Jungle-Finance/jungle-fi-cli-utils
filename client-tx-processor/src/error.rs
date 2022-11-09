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

/// Prints the transaction logs for failed preflight simulations.
/// Otherwise just prints the error.
/// Returns the error back out for any further desired processing.
#[allow(dead_code)]
pub fn maybe_print_preflight_simulation_logs(
    err: solana_client::client_error::ClientError
) -> solana_client::client_error::ClientError {
    if let ClientErrorKind::RpcError(err) = &err.kind {
        if let RpcError::RpcResponseError { data, .. } = err {
            // print the transaction logs for a failed pre-flight simulation
            if let RpcResponseErrorData::SendTransactionPreflightFailure(
                result
            ) = data {
                if let Some(logs) = &result.logs {
                    logs.iter().for_each(|e| println!("{}", e))
                }
            }
        }
    }
    err
}
