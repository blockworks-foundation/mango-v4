use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::*;

use crate::priority_fees::*;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum PriorityFeeStyleArg {
    None,
    Fixed,
    LiteRpcCuPercentileEma,
}

#[derive(clap::Args, Debug, Clone)]
pub struct PriorityFeeArgs {
    /// choose prio fee style
    #[clap(long, env, value_enum, default_value = "none")]
    prioritization_style: PriorityFeeStyleArg,

    /// prioritize each transaction with this many microlamports/cu
    ///
    /// for dynamic prio styles, this is the fallback value
    #[clap(long, env, default_value = "0")]
    prioritization_micro_lamports: u64,

    #[clap(long, env, default_value = "50")]
    prioritization_ema_percentile: u8,

    #[clap(long, env, default_value = "0.2")]
    prioritization_ema_alpha: f64,
}

impl PriorityFeeArgs {
    pub fn make_prio_provider(
        &self,
        lite_rpc_url: String,
    ) -> anyhow::Result<(Option<Arc<dyn PriorityFeeProvider>>, Vec<JoinHandle<()>>)> {
        let prio_style;
        if self.prioritization_micro_lamports > 0
            && self.prioritization_style == PriorityFeeStyleArg::None
        {
            info!("forcing prioritization-style to fixed, since prioritization-micro-lamports was set");
            prio_style = PriorityFeeStyleArg::Fixed;
        } else {
            prio_style = self.prioritization_style;
        }

        Ok(match prio_style {
            PriorityFeeStyleArg::None => (None, vec![]),
            PriorityFeeStyleArg::Fixed => (
                Some(Arc::new(FixedPriorityFeeProvider::new(
                    self.prioritization_micro_lamports,
                ))),
                vec![],
            ),
            PriorityFeeStyleArg::LiteRpcCuPercentileEma => {
                if lite_rpc_url.is_empty() {
                    anyhow::bail!("cannot use recent-cu-percentile-ema prioritization style without a lite-rpc url");
                }
                let (block_prio_broadcaster, block_prio_job) =
                    run_broadcast_from_websocket_feed(lite_rpc_url);
                let (prio_fee_provider, prio_fee_provider_job) =
                    CuPercentileEmaPriorityFeeProvider::run(
                        EmaPriorityFeeProviderConfig::builder()
                            .percentile(75)
                            .fallback_prio(self.prioritization_micro_lamports)
                            .alpha(self.prioritization_ema_alpha)
                            .percentile(self.prioritization_ema_percentile)
                            .build()
                            .unwrap(),
                        &block_prio_broadcaster,
                    );
                (
                    Some(prio_fee_provider),
                    vec![block_prio_job, prio_fee_provider_job],
                )
            }
        })
    }
}
