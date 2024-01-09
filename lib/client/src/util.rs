use solana_client::{
    client_error::Result as ClientResult, rpc_client::RpcClient, rpc_request::RpcError,
};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{
    clock::Slot, commitment_config::CommitmentConfig, signature::Signature,
    transaction::uses_durable_nonce,
};

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anyhow::Context;
use std::{thread, time};

/// Some Result<> types don't convert to anyhow::Result nicely. Force them through stringification.
pub trait AnyhowWrap {
    type Value;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value>;
}

impl<T, E: std::fmt::Debug> AnyhowWrap for Result<T, E> {
    type Value = T;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value> {
        self.map_err(|err| anyhow::anyhow!("{:?}", err))
    }
}

/// Push to an async_channel::Sender and ignore if the channel is full
pub trait AsyncChannelSendUnlessFull<T> {
    /// Send a message if the channel isn't full
    fn send_unless_full(&self, msg: T) -> Result<(), async_channel::SendError<T>>;
}

impl<T> AsyncChannelSendUnlessFull<T> for async_channel::Sender<T> {
    fn send_unless_full(&self, msg: T) -> Result<(), async_channel::SendError<T>> {
        use async_channel::*;
        match self.try_send(msg) {
            Ok(()) => Ok(()),
            Err(TrySendError::Closed(msg)) => Err(async_channel::SendError(msg)),
            Err(TrySendError::Full(_)) => Ok(()),
        }
    }
}

/// Like tokio::time::interval(), but with Delay as default MissedTickBehavior
///
/// The default (Burst) means that if the time between tick() calls is longer
/// than `period` there'll be a burst of catch-up ticks.
///
/// This Interval guarantees that when tick() returns, at least `period` will have
/// elapsed since the last return. That way it's more appropriate for jobs that
/// don't need to catch up.
pub fn delay_interval(period: std::time::Duration) -> tokio::time::Interval {
    let mut interval = tokio::time::interval(period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval
}

/// A copy of RpcClient::send_and_confirm_transaction that returns the slot the
/// transaction confirmed in.
pub fn send_and_confirm_transaction(
    rpc_client: &RpcClient,
    transaction: &Transaction,
) -> ClientResult<(Signature, Slot)> {
    const SEND_RETRIES: usize = 1;
    const GET_STATUS_RETRIES: usize = usize::MAX;

    'sending: for _ in 0..SEND_RETRIES {
        let signature = rpc_client.send_transaction(transaction)?;

        let recent_blockhash = if uses_durable_nonce(transaction).is_some() {
            let (recent_blockhash, ..) =
                rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::processed())?;
            recent_blockhash
        } else {
            transaction.message.recent_blockhash
        };

        for status_retry in 0..GET_STATUS_RETRIES {
            let response = rpc_client.get_signature_statuses(&[signature])?.value;
            match response[0]
                .clone()
                .filter(|result| result.satisfies_commitment(rpc_client.commitment()))
            {
                Some(tx_status) => {
                    return if let Some(e) = tx_status.err {
                        Err(e.into())
                    } else {
                        Ok((signature, tx_status.slot))
                    };
                }
                None => {
                    if !rpc_client
                        .is_blockhash_valid(&recent_blockhash, CommitmentConfig::processed())?
                    {
                        // Block hash is not found by some reason
                        break 'sending;
                    } else if cfg!(not(test))
                        // Ignore sleep at last step.
                        && status_retry < GET_STATUS_RETRIES
                    {
                        // Retry twice a second
                        thread::sleep(time::Duration::from_millis(500));
                        continue;
                    }
                }
            }
        }
    }

    Err(RpcError::ForUser(
        "unable to confirm transaction. \
            This can happen in situations such as transaction expiration \
            and insufficient fee-payer funds"
            .to_string(),
    )
    .into())
}

/// Convenience function used in binaries to set up the fmt tracing_subscriber,
/// with cololring enabled only if logging to a terminal and with EnvFilter.
pub fn tracing_subscriber_init() {
    let format = tracing_subscriber::fmt::format().with_ansi(atty::is(atty::Stream::Stdout));

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .event_format(format)
        .init();
}

pub async fn http_error_handling<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> anyhow::Result<T> {
    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("awaiting body of http request")?;
    if !status.is_success() {
        anyhow::bail!("http request failed, status: {status}, body: {response_text}");
    }
    serde_json::from_str::<T>(&response_text)
        .with_context(|| format!("response has unexpected format, body: {response_text}"))
}

pub fn to_readonly_account_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta {
        pubkey,
        is_writable: false,
        is_signer: false,
    }
}

pub fn to_writable_account_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta {
        pubkey,
        is_writable: true,
        is_signer: false,
    }
}

#[derive(Default, Clone)]
pub struct PreparedInstructions {
    pub instructions: Vec<Instruction>,
    pub cu: u32,
}

impl PreparedInstructions {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
            cu: 0,
        }
    }

    pub fn from_vec(instructions: Vec<Instruction>, cu: u32) -> Self {
        Self { instructions, cu }
    }

    pub fn from_single(instruction: Instruction, cu: u32) -> Self {
        Self {
            instructions: vec![instruction],
            cu,
        }
    }

    pub fn push(&mut self, ix: Instruction, cu: u32) {
        self.instructions.push(ix);
        self.cu += cu;
    }

    pub fn append(&mut self, mut other: Self) {
        self.instructions.append(&mut other.instructions);
        self.cu += other.cu;
    }

    pub fn to_instructions(self) -> Vec<Instruction> {
        let mut ixs = self.instructions;
        ixs.insert(0, ComputeBudgetInstruction::set_compute_unit_limit(self.cu));
        ixs
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    pub fn clear(&mut self) {
        self.instructions.clear();
        self.cu = 0;
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }
}
