use crate::trigger_tcs;
use anchor_lang::prelude::Pubkey;
use clap::Parser;
use mango_v4_client::{jupiter, priority_fees_cli};
use std::collections::HashSet;

#[derive(Parser, Debug)]
#[clap()]
pub(crate) struct CliDotenv {
    // When --dotenv <file> is passed, read the specified dotenv file before parsing args
    #[clap(long)]
    pub(crate) dotenv: std::path::PathBuf,

    pub(crate) remaining_args: Vec<std::ffi::OsString>,
}

// Prefer "--rebalance false" over "--no-rebalance" because it works
// better with REBALANCE=false env values.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoolArg {
    True,
    False,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JupiterVersionArg {
    Mock,
    V6,
}

impl From<JupiterVersionArg> for jupiter::Version {
    fn from(a: JupiterVersionArg) -> Self {
        match a {
            JupiterVersionArg::Mock => jupiter::Version::Mock,
            JupiterVersionArg::V6 => jupiter::Version::V6,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TcsMode {
    BorrowBuy,
    SwapSellIntoBuy,
    SwapCollateralIntoBuy,
}

impl From<TcsMode> for trigger_tcs::Mode {
    fn from(a: TcsMode) -> Self {
        match a {
            TcsMode::BorrowBuy => trigger_tcs::Mode::BorrowBuyToken,
            TcsMode::SwapSellIntoBuy => trigger_tcs::Mode::SwapSellIntoBuy,
            TcsMode::SwapCollateralIntoBuy => trigger_tcs::Mode::SwapCollateralIntoBuy,
        }
    }
}

pub(crate) fn cli_to_hashset<T: Eq + std::hash::Hash + From<u16>>(
    str_list: Option<Vec<u16>>,
) -> HashSet<T> {
    return str_list
        .map(|v| v.iter().map(|x| T::from(*x)).collect::<HashSet<T>>())
        .unwrap_or_default();
}

#[derive(Parser)]
#[clap()]
pub struct Cli {
    #[clap(short, long, env)]
    pub(crate) rpc_url: String,

    #[clap(long, env, value_delimiter = ';')]
    pub(crate) override_send_transaction_url: Option<Vec<String>>,

    #[clap(long, env)]
    pub(crate) liqor_mango_account: Pubkey,

    #[clap(long, env)]
    pub(crate) liqor_owner: String,

    #[clap(long, env, default_value = "1000")]
    pub(crate) check_interval_ms: u64,

    #[clap(long, env, default_value = "300")]
    pub(crate) snapshot_interval_secs: u64,

    // how often do we refresh token swap route/prices
    #[clap(long, env, default_value = "30")]
    pub(crate) token_swap_refresh_interval_secs: u64,

    /// how many getMultipleAccounts requests to send in parallel
    #[clap(long, env, default_value = "10")]
    pub(crate) parallel_rpc_requests: usize,

    /// typically 100 is the max number of accounts getMultipleAccounts will retrieve at once
    #[clap(long, env, default_value = "100")]
    pub(crate) get_multiple_accounts_count: usize,

    /// liquidator health ratio should not fall below this value
    #[clap(long, env, default_value = "50")]
    pub(crate) min_health_ratio: f64,

    /// if rebalancing is enabled
    ///
    /// typically only disabled for tests where swaps are unavailable
    #[clap(long, env, value_enum, default_value = "true")]
    pub(crate) rebalance: BoolArg,

    /// max slippage to request on swaps to rebalance spot tokens
    #[clap(long, env, default_value = "100")]
    pub(crate) rebalance_slippage_bps: u64,

    /// tokens to not rebalance (in addition to USDC=0); use a comma separated list of token index
    #[clap(long, env, value_parser, value_delimiter = ',')]
    pub(crate) rebalance_skip_tokens: Option<Vec<u16>>,

    /// When closing borrows, the rebalancer can't close token positions exactly.
    /// Instead it purchases too much and then gets rid of the excess in a second step.
    /// If this is 0.05, then it'll swap borrow_value * (1 + 0.05) quote token into borrow token.
    #[clap(long, env, default_value = "0.05")]
    pub(crate) rebalance_borrow_settle_excess: f64,

    #[clap(long, env, default_value = "30")]
    pub(crate) rebalance_refresh_timeout_secs: u64,

    /// if taking tcs orders is enabled
    ///
    /// typically only disabled for tests where swaps are unavailable
    #[clap(long, env, value_enum, default_value = "true")]
    pub(crate) take_tcs: BoolArg,

    /// profit margin at which to take tcs orders
    #[clap(long, env, default_value = "0.0005")]
    pub(crate) tcs_profit_fraction: f64,

    /// control how tcs triggering provides buy tokens
    #[clap(long, env, value_enum, default_value = "swap-sell-into-buy")]
    pub(crate) tcs_mode: TcsMode,

    /// largest tcs amount to trigger in one transaction, in dollar
    #[clap(long, env, default_value = "1000.0")]
    pub(crate) tcs_max_trigger_amount: f64,

    /// Minimum fraction of max_buy to buy for success when triggering,
    /// useful in conjunction with jupiter swaps in same tx to avoid over-buying.
    ///
    /// Can be set to 0 to allow executions of any size.
    #[clap(long, env, default_value = "0.7")]
    pub(crate) tcs_min_buy_fraction: f64,

    #[clap(flatten)]
    pub(crate) prioritization_fee_cli: priority_fees_cli::PriorityFeeArgs,

    /// url to the lite-rpc websocket, optional
    #[clap(long, env, default_value = "")]
    pub(crate) lite_rpc_url: String,

    /// compute limit requested for liquidation instructions
    #[clap(long, env, default_value = "250000")]
    pub(crate) compute_limit_for_liquidation: u32,

    /// compute limit requested for tcs trigger instructions
    #[clap(long, env, default_value = "300000")]
    pub(crate) compute_limit_for_tcs: u32,

    /// control which version of jupiter to use
    #[clap(long, env, value_enum, default_value = "v6")]
    pub(crate) jupiter_version: JupiterVersionArg,

    /// override the url to jupiter v6
    #[clap(long, env, default_value = "https://quote-api.jup.ag/v6")]
    pub(crate) jupiter_v6_url: String,

    /// provide a jupiter token, currently only for jup v6
    #[clap(long, env, default_value = "")]
    pub(crate) jupiter_token: String,

    /// size of the swap to quote via jupiter to get slippage info, in dollar
    /// should be larger than tcs_max_trigger_amount
    #[clap(long, env, default_value = "1000.0")]
    pub(crate) jupiter_swap_info_amount: f64,

    /// report liquidator's existence and pubkey
    #[clap(long, env, value_enum, default_value = "true")]
    pub(crate) telemetry: BoolArg,

    /// liquidation refresh timeout in secs
    #[clap(long, env, default_value = "30")]
    pub(crate) liquidation_refresh_timeout_secs: u8,

    /// tokens to exclude for liquidation/tcs (never liquidate any pair where base or quote is in this list)
    #[clap(long, env, value_parser, value_delimiter = ' ')]
    pub(crate) forbidden_tokens: Option<Vec<u16>>,

    /// tokens to allow for liquidation/tcs (only liquidate a pair if base or quote is in this list)
    /// when empty, allows all pairs
    #[clap(long, env, value_parser, value_delimiter = ' ')]
    pub(crate) only_allow_tokens: Option<Vec<u16>>,

    /// perp market to exclude for liquidation
    #[clap(long, env, value_parser, value_delimiter = ' ')]
    pub(crate) liquidation_forbidden_perp_markets: Option<Vec<u16>>,

    /// perp market to allow for liquidation (only liquidate if is in this list)
    /// when empty, allows all pairs
    #[clap(long, env, value_parser, value_delimiter = ' ')]
    pub(crate) liquidation_only_allow_perp_markets: Option<Vec<u16>>,

    /// how long should it wait before logging an oracle error again (for the same token)
    #[clap(long, env, default_value = "30")]
    pub(crate) skip_oracle_error_in_logs_duration_secs: u64,
}
