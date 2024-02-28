use crate::configuration::Configuration;
use crate::processors::data::DataEvent;
use chrono::Utc;
use mango_v4::health::HealthType;
use mango_v4_client::chain_data::AccountFetcher;
use mango_v4_client::{chain_data, health_cache, FallbackOracleConfig, MangoGroupContext};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::task::JoinHandle;
use tracing::warn;

pub struct HealthProcessor {
    pub channel: tokio::sync::broadcast::Sender<HealthEvent>,
    pub job: JoinHandle<()>,
}

#[derive(Clone, Debug)]
pub struct HealthEvent {
    pub computed_at: chrono::DateTime<Utc>,
    pub components: Vec<HealthComponent>,
}

#[derive(Clone, Debug)]
pub struct HealthComponent {
    pub account: Pubkey,
    pub value: Option<HealthComponentValue>,
}

#[derive(Clone, Debug)]
pub struct HealthComponentValue {
    pub maintenance_ratio: f64,
    pub initial_health: f64,
    pub maintenance_health: f64,
    pub liquidation_end_health: f64,
    pub is_being_liquidated: bool,
}

impl HealthProcessor {
    pub async fn init(
        data_sender: &tokio::sync::broadcast::Sender<DataEvent>,
        chain_data: Arc<RwLock<chain_data::ChainData>>,
        configuration: &Configuration,
        exit: Arc<AtomicBool>,
    ) -> anyhow::Result<HealthProcessor> {
        let (sender, _) = tokio::sync::broadcast::channel(8192);
        let sender_clone = sender.clone();
        let mut data = data_sender.subscribe();
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

                        if sender_clone.receiver_count() == 0 {
                            continue;
                        }

                        if snapshot_received && last_recompute.elapsed() >= recompute_interval {
                            last_recompute = Instant::now();

                            let health_event = Self::compute_health(&mango_group_context,
                                &account_fetcher,
                                &accounts).await;

                            let res = sender_clone.send(health_event);
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

        let result = HealthProcessor {
            channel: sender,
            job,
        };

        Ok(result)
    }

    async fn compute_health(
        mango_group_context: &MangoGroupContext,
        account_fetcher: &AccountFetcher,
        accounts: &HashSet<Pubkey>,
    ) -> HealthEvent {
        let computed_at = Utc::now();
        let mut components = Vec::new();

        for account in accounts {
            let value =
                Self::compute_account_health(&mango_group_context, account_fetcher, &account).await;

            components.push({
                HealthComponent {
                    account: *account,
                    value: value.ok(),
                }
            })
        }

        HealthEvent {
            computed_at,
            components,
        }
    }

    async fn compute_account_health(
        mango_group_context: &&MangoGroupContext,
        account_fetcher: &AccountFetcher,
        account: &Pubkey,
    ) -> anyhow::Result<HealthComponentValue> {
        let mango_account = account_fetcher.fetch_mango_account(account)?;
        let health_cache = health_cache::new(
            &mango_group_context,
            &FallbackOracleConfig::Never,
            &*account_fetcher,
            &mango_account,
        )
        .await?;

        let res = HealthComponentValue {
            maintenance_ratio: health_cache.health_ratio(HealthType::Maint).to_num(),
            initial_health: health_cache.health(HealthType::Init).to_num(),
            maintenance_health: health_cache.health(HealthType::Maint).to_num(),
            liquidation_end_health: health_cache.health(HealthType::LiquidationEnd).to_num(),
            is_being_liquidated: mango_account.fixed.being_liquidated(),
        };

        Ok(res)
    }
}
