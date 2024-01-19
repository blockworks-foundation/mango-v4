use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_client::rpc_request::RpcError;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::TransactionStatus;

use crate::util::delay_interval;
use std::time::Duration;

#[derive(thiserror::Error, Debug)]
pub enum WaitForTransactionConfirmationError {
    #[error("blockhash has expired")]
    BlockhashExpired,
    #[error("timeout expired")]
    Timeout,
    #[error("client error: {0:?}")]
    ClientError(#[from] solana_client::client_error::ClientError),
}

#[derive(Clone, Debug, Builder)]
#[builder(default)]
pub struct RpcConfirmTransactionConfig {
    /// If none, defaults to the RpcClient's configured default commitment.
    pub commitment: Option<CommitmentConfig>,

    /// Time after which to start checking for blockhash expiry.
    pub recent_blockhash_initial_timeout: Duration,

    /// Interval between signature status queries.
    pub signature_status_interval: Duration,

    /// If none, there's no timeout. The confirmation will still abort eventually
    /// when the blockhash expires.
    pub timeout: Option<Duration>,
}

impl Default for RpcConfirmTransactionConfig {
    fn default() -> Self {
        Self {
            commitment: None,
            recent_blockhash_initial_timeout: Duration::from_secs(5),
            signature_status_interval: Duration::from_millis(500),
            timeout: None,
        }
    }
}

impl RpcConfirmTransactionConfig {
    pub fn builder() -> RpcConfirmTransactionConfigBuilder {
        RpcConfirmTransactionConfigBuilder::default()
    }
}

/// Wait for `signature` to be confirmed at `commitment` or until either
/// - `recent_blockhash` is so old that the tx can't be confirmed _and_
///   `blockhash_initial_timeout` is reached
/// - the `signature_status_timeout` is reached
/// While waiting, query for confirmation every `signature_status_interval`
///
/// NOTE: RpcClient::config contains confirm_transaction_initial_timeout which is the
/// same as blockhash_initial_timeout. Unfortunately the former is private.
///
/// Returns:
/// - blockhash and blockhash_initial_timeout expired -> BlockhashExpired error
/// - signature_status_timeout expired -> Timeout error (possibly just didn't reach commitment in time?)
/// - any rpc error -> ClientError error
/// - confirmed at commitment -> ok(slot, opt<tx_error>)
pub async fn wait_for_transaction_confirmation(
    rpc_client: &RpcClientAsync,
    signature: &Signature,
    recent_blockhash: &solana_sdk::hash::Hash,
    config: &RpcConfirmTransactionConfig,
) -> Result<TransactionStatus, WaitForTransactionConfirmationError> {
    let mut signature_status_interval = delay_interval(config.signature_status_interval);
    let commitment = config.commitment.unwrap_or(rpc_client.commitment());

    let start = std::time::Instant::now();
    let is_timed_out = || config.timeout.map(|t| start.elapsed() > t).unwrap_or(false);
    loop {
        signature_status_interval.tick().await;
        if is_timed_out() {
            return Err(WaitForTransactionConfirmationError::Timeout);
        }

        let statuses = rpc_client
            .get_signature_statuses(&[signature.clone()])
            .await?;
        let status_opt = match statuses.value.into_iter().next() {
            Some(v) => v,
            None => {
                return Err(WaitForTransactionConfirmationError::ClientError(
                    RpcError::ParseError(
                        "must contain an entry for each requested signature".into(),
                    )
                    .into(),
                ));
            }
        };

        // If the tx isn't seen at all (not even processed), check blockhash expiry
        if status_opt.is_none() {
            if start.elapsed() > config.recent_blockhash_initial_timeout {
                let blockhash_is_valid = rpc_client
                    .is_blockhash_valid(recent_blockhash, CommitmentConfig::processed())
                    .await?;
                if !blockhash_is_valid {
                    return Err(WaitForTransactionConfirmationError::BlockhashExpired);
                }
            }
            continue;
        }

        let status = status_opt.unwrap();
        if status.satisfies_commitment(commitment) {
            return Ok(status);
        }
    }
}
