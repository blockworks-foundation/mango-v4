use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use itertools::Itertools;
use tracing::*;

use mango_v4::state::TokenIndex;
use mango_v4_client::jupiter;
use mango_v4_client::MangoClient;

pub struct Config {
    pub quote_index: TokenIndex,
    pub quote_amount: u64,
    pub jupiter_version: jupiter::Version,
}

#[derive(Clone, Default)]
pub struct TokenSwapInfo {
    /// multiplier to the oracle price for executing a buy, so 1.5 would mean buying 50% over oracle price
    pub buy_over_oracle: f64,
    /// multiplier to the oracle price for executing a sell,
    /// but with the price inverted, so values > 1 mean a worse deal than oracle price
    pub sell_over_oracle: f64,
}

/// Track the buy/sell slippage for tokens
///
/// Needed to evaluate whether a token conditional swap premium might be good enough
/// without having to query each time.
pub struct TokenSwapInfoUpdater {
    mango_client: Arc<MangoClient>,
    swap_infos: RwLock<HashMap<TokenIndex, TokenSwapInfo>>,
    config: Config,
}

impl TokenSwapInfoUpdater {
    pub fn new(mango_client: Arc<MangoClient>, config: Config) -> Self {
        Self {
            mango_client,
            swap_infos: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub fn mango_client(&self) -> &Arc<MangoClient> {
        &self.mango_client
    }

    fn update(&self, token_index: TokenIndex, slippage: TokenSwapInfo) {
        let mut lock = self.swap_infos.write().unwrap();
        let entry = lock.entry(token_index).or_default();
        *entry = slippage;
    }

    pub fn swap_info(&self, token_index: TokenIndex) -> Option<TokenSwapInfo> {
        let lock = self.swap_infos.read().unwrap();
        lock.get(&token_index).cloned()
    }

    /// oracle price is how many "in" tokens to pay for one "out" token
    fn price_over_oracle(oracle_price: f64, route: &jupiter::Quote) -> anyhow::Result<f64> {
        let in_amount = route.in_amount as f64;
        let out_amount = route.out_amount as f64;
        let actual_price = in_amount / out_amount;
        Ok(actual_price / oracle_price)
    }

    pub async fn update_one(&self, token_index: TokenIndex) -> anyhow::Result<()> {
        // since we're only quoting, the slippage does not matter
        let slippage = 100;

        let quote_index = self.config.quote_index;
        if token_index == quote_index {
            self.update(quote_index, TokenSwapInfo::default());
            return Ok(());
        }

        let token_mint = self.mango_client.context.mint_info(token_index).mint;
        let quote_mint = self.mango_client.context.mint_info(quote_index).mint;

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
        let quote_per_token_price = token_price / quote_price;
        let token_per_quote_price = quote_price / token_price;

        let token_amount = (self.config.quote_amount as f64 * token_per_quote_price) as u64;
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

        let buy_over_oracle = Self::price_over_oracle(quote_per_token_price, &buy_route)?;
        let sell_over_oracle = Self::price_over_oracle(token_per_quote_price, &sell_route)?;

        self.update(
            token_index,
            TokenSwapInfo {
                buy_over_oracle,
                sell_over_oracle,
            },
        );
        Ok(())
    }

    pub fn log_all(&self) {
        let mut tokens = self
            .mango_client
            .context
            .token_indexes_by_name
            .clone()
            .into_iter()
            .collect_vec();
        tokens.sort_by(|a, b| a.0.cmp(&b.0));
        let infos = self.swap_infos.read().unwrap();

        let mut msg = String::new();
        for (token, token_index) in tokens {
            let info = infos
                .get(&token_index)
                .map(|info| {
                    format!(
                        "buy {}, sell {}",
                        info.buy_over_oracle, info.sell_over_oracle
                    )
                })
                .unwrap_or_else(|| "no data".into());
            msg.push_str(&format!("token {token}, {info}"));
        }
        trace!("swap infos:{}", msg);
    }
}
