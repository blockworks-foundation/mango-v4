use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use itertools::Itertools;
use mango_v4_client::error_tracking::ErrorTracking;
use tracing::*;

use mango_v4::state::TokenIndex;
use mango_v4_client::jupiter;
use mango_v4_client::MangoClient;

pub struct Config {
    pub quote_index: TokenIndex,

    /// Size in quote_index-token native tokens to quote.
    pub quote_amount: u64,

    pub jupiter_version: jupiter::Version,
}

#[derive(Clone)]
pub struct TokenSwapInfo {
    pub last_update: std::time::SystemTime,

    // in USDC per token, not the literal oracle price (which is in USD per token)
    pub quote_per_token_oracle: f64,
    pub quote_per_token_buy: f64,
    pub quote_per_token_sell: f64,
}

impl TokenSwapInfo {
    /// multiplier to the oracle price for executing a buy, so 1.5 would mean buying 50% over oracle price
    pub fn buy_over_oracle(&self) -> f64 {
        self.quote_per_token_buy / self.quote_per_token_oracle
    }

    /// multiplier to the oracle price for executing a sell,
    /// but with the price inverted, so values > 1 mean a worse deal than oracle price
    pub fn sell_over_oracle(&self) -> f64 {
        self.quote_per_token_oracle / self.quote_per_token_sell
    }
}

struct TokenSwapInfoState {
    swap_infos: HashMap<TokenIndex, TokenSwapInfo>,
    errors: ErrorTracking<TokenIndex, &'static str>,
}

/// Track the buy/sell slippage for tokens
///
/// Needed to evaluate whether a token conditional swap premium might be good enough
/// without having to query each time.
pub struct TokenSwapInfoUpdater {
    mango_client: Arc<MangoClient>,
    state: RwLock<TokenSwapInfoState>,
    config: Config,
}

const ERROR_TYPE: &'static str = "tsi";

impl TokenSwapInfoUpdater {
    pub fn new(mango_client: Arc<MangoClient>, config: Config) -> Self {
        Self {
            mango_client,
            state: RwLock::new(TokenSwapInfoState {
                swap_infos: HashMap::new(),
                errors: ErrorTracking::builder().build().unwrap(),
            }),
            config,
        }
    }

    pub fn mango_client(&self) -> &Arc<MangoClient> {
        &self.mango_client
    }

    fn update(&self, token_index: TokenIndex, slippage: TokenSwapInfo) {
        let mut lock = self.state.write().unwrap();
        lock.swap_infos.insert(token_index, slippage);
    }

    pub fn swap_info(&self, token_index: TokenIndex) -> Option<TokenSwapInfo> {
        let lock = self.state.read().unwrap();
        lock.swap_infos.get(&token_index).cloned()
    }

    fn in_per_out_price(route: &jupiter::Quote) -> f64 {
        let in_amount = route.in_amount as f64;
        let out_amount = route.out_amount as f64;
        in_amount / out_amount
    }

    pub async fn update_one(&self, token_index: TokenIndex) {
        {
            let lock = self.state.read().unwrap();
            if lock
                .errors
                .had_too_many_errors(ERROR_TYPE, &token_index, std::time::Instant::now())
                .is_some()
            {
                return;
            }
        }

        if let Err(err) = self.try_update_one(token_index).await {
            let mut lock = self.state.write().unwrap();
            lock.errors
                .record(ERROR_TYPE, &token_index, err.to_string());
        }
    }

    async fn try_update_one(&self, token_index: TokenIndex) -> anyhow::Result<()> {
        // since we're only quoting, the slippage does not matter
        let slippage = 100;

        let quote_index = self.config.quote_index;
        if token_index == quote_index {
            self.update(
                quote_index,
                TokenSwapInfo {
                    last_update: std::time::SystemTime::now(),
                    quote_per_token_oracle: 1.0,
                    quote_per_token_buy: 1.0,
                    quote_per_token_sell: 1.0,
                },
            );
            return Ok(());
        }

        let token_mint = self.mango_client.context.token(token_index).mint;
        let quote_mint = self.mango_client.context.token(quote_index).mint;

        // these prices are in USD, which doesn't exist on chain
        let token_price = self
            .mango_client
            .bank_oracle_price(token_index)
            .await?
            .to_num::<f64>();
        let quote_price = self
            .mango_client
            .bank_oracle_price(quote_index)
            .await?
            .to_num::<f64>();

        // prices for the pair
        let quote_per_token_oracle = token_price / quote_price;
        let token_per_quote_oracle = quote_price / token_price;

        let token_amount = (self.config.quote_amount as f64 * token_per_quote_oracle) as u64;
        let sell_route = self
            .mango_client
            .jupiter()
            .quote(
                token_mint,
                quote_mint,
                token_amount,
                slippage,
                false,
                self.config.jupiter_version,
            )
            .await?;
        let buy_route = self
            .mango_client
            .jupiter()
            .quote(
                quote_mint,
                token_mint,
                self.config.quote_amount,
                slippage,
                false,
                self.config.jupiter_version,
            )
            .await?;

        let quote_per_token_buy = Self::in_per_out_price(&buy_route);
        let token_per_quote_sell = Self::in_per_out_price(&sell_route);

        self.update(
            token_index,
            TokenSwapInfo {
                last_update: std::time::SystemTime::now(),
                quote_per_token_oracle,
                quote_per_token_buy,
                quote_per_token_sell: 1.0 / token_per_quote_sell,
            },
        );
        Ok(())
    }

    pub fn log_all(&self) {
        {
            let mut lock = self.state.write().unwrap();
            lock.errors.update();
        }

        let mut tokens = self
            .mango_client
            .context
            .token_indexes_by_name
            .clone()
            .into_iter()
            .collect_vec();
        tokens.sort_by(|a, b| a.0.cmp(&b.0));
        let lock = self.state.read().unwrap();

        let mut msg = String::new();
        for (token, token_index) in tokens {
            let info = lock
                .swap_infos
                .get(&token_index)
                .map(|info| {
                    format!(
                        "oracle {}, buy {}, sell {}",
                        info.quote_per_token_oracle,
                        info.quote_per_token_buy,
                        info.quote_per_token_sell
                    )
                })
                .unwrap_or_else(|| "no data".into());
            msg.push_str(&format!("token {token}, {info}"));
        }
        trace!("swap infos:{}", msg);
    }
}
