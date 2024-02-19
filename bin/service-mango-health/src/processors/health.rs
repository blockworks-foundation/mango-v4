use crate::configuration::Configuration;
use crate::processors::data::DataEvent;
use fixed::types::I80F48;
use log::warn;
use mango_v4::accounts_zerocopy::LoadZeroCopy;
use mango_v4::health::HealthType;
use mango_v4_client::chain_data::AccountFetcher;
use mango_v4_client::{chain_data, health_cache, FallbackOracleConfig, MangoGroupContext};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::time::Interval;

pub struct HealthProcessor {
    pub receiver: async_channel::Receiver<HealthEvent>,
    pub job: JoinHandle<()>,
}

#[derive(Clone, Debug)]
pub struct HealthEvent {
    pub computed_at: Instant,
    pub components: Vec<HealthComponent>,
}

#[derive(Clone, Debug)]
pub struct HealthComponent {
    pub account: Pubkey,
    pub health_ratio: I80F48,
}

impl HealthProcessor {
    pub async fn init(
        data: async_channel::Receiver<DataEvent>,
        chain_data: Arc<RwLock<chain_data::ChainData>>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<HealthProcessor> {
        let (sender, receiver) = async_channel::unbounded::<HealthEvent>();
        let mut accounts = HashSet::<Pubkey>::new();
        let mut snapshot_received = false;
        let mut last_recompute = Instant::now();
        let recompute_interval = std::time::Duration::from_millis(
            configuration.computing_configuration.recompute_interval_ms,
        );

        let account_fetcher = chain_data::AccountFetcher {
            chain_data: chain_data.clone(),
            rpc: RpcClient::new(configuration.rpc_http_url.clone()),
        };

        let mango_group_context = MangoGroupContext::new_from_rpc(
            &account_fetcher.rpc,
            Pubkey::from_str(&configuration.mango_group)?,
        )
        .await?;

        let job = tokio::spawn(async move {
            loop {
                if exit.load(Ordering::Relaxed) {
                    warn!("shutting down health processor...");
                    break;
                }

                tokio::select! {
                    Ok(msg) = data.recv() => {
                        match msg {
                            DataEvent::AccountUpdate(upd) => {
                                accounts.insert(upd.account);
                            },
                            DataEvent::Snapshot(snap) => {
                                for account in snap.accounts {
                                    accounts.insert(account);
                                }
                                snapshot_received = true;
                            },
                            DataEvent::Other => {
                            }
                        }

                        if snapshot_received && last_recompute.elapsed() >= recompute_interval {
                            last_recompute = Instant::now();
                            let health_event_res = Self::compute_health(&mango_group_context,
                                &account_fetcher,
                                &accounts).await;
                            if health_event_res.is_err(){
                                // TODO FAS Log ? Fail ?
                                warn!("Error while fetching health: {}", health_event_res.unwrap_err());
                                continue;
                            }

                            let res = sender.try_send(health_event_res.unwrap());
                            if res.is_err() {
                                break;
                            }
                        }
                    },
                    else => {
                        warn!("data update channel err");
                        break;
                    }
                }
            }
        });

        let result = HealthProcessor { receiver, job };

        Ok(result)
    }

    async fn compute_health(
        mango_group_context: &MangoGroupContext,
        account_fetcher: &AccountFetcher,
        accounts: &HashSet<Pubkey>,
    ) -> anyhow::Result<HealthEvent> {
        let computed_at = Instant::now();
        let mut components = Vec::new();

        for account in accounts {
            let mango_account = account_fetcher.fetch_mango_account(account)?;
            let health_cache = health_cache::new(
                &mango_group_context,
                &FallbackOracleConfig::Never,
                &*account_fetcher,
                &mango_account,
            )
            .await?;

            components.push({
                HealthComponent {
                    account: *account,
                    health_ratio: health_cache.health_ratio(HealthType::Maint),
                }
            })
        }

        Ok(HealthEvent {
            computed_at,
            components,
        })
    }
}