use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use mango_v4::state::TokenIndex;
use mango_v4_client::jupiter::QueryRoute;
use mango_v4_client::{JupiterSwapMode, MangoClient};

use crate::util;

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
    slippage_by_token: RwLock<HashMap<TokenIndex, TokenSwapInfo>>,
    quote_amount: u64,
    mock_jupiter: bool,
}

impl TokenSwapInfoUpdater {
    pub fn new(mango_client: Arc<MangoClient>) -> Self {
        Self {
            mango_client,
            slippage_by_token: RwLock::new(HashMap::new()),
            quote_amount: 1_000_000_000, // TODO: config
            mock_jupiter: false,
        }
    }

    pub fn mango_client(&self) -> &Arc<MangoClient> {
        &self.mango_client
    }

    fn update(&self, token_index: TokenIndex, slippage: TokenSwapInfo) {
        let mut lock = self.slippage_by_token.write().unwrap();
        let entry = lock.entry(token_index).or_default();
        *entry = slippage;
    }

    pub fn swap_info(&self, token_index: TokenIndex) -> Option<TokenSwapInfo> {
        let lock = self.slippage_by_token.read().unwrap();
        lock.get(&token_index).cloned()
    }

    /// oracle price is how many "in" tokens to pay for one "out" token
    fn price_over_oracle(oracle_price: f64, route: QueryRoute) -> anyhow::Result<f64> {
        let in_amount = route.in_amount.parse::<f64>()?;
        let out_amount = route.out_amount.parse::<f64>()?;
        let actual_price = in_amount / out_amount;
        log::info!("check actual {actual_price}, oralce {oracle_price}");
        Ok(actual_price / oracle_price)
    }

    pub async fn update_one(&self, token_index: TokenIndex) -> anyhow::Result<()> {
        let quote_index = 0;
        let slippage = 100;
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

        let token_amount = (self.quote_amount as f64 * token_per_quote_price) as u64;
        let sell_route = util::jupiter_route(
            &self.mango_client,
            token_mint,
            quote_mint,
            token_amount,
            slippage,
            JupiterSwapMode::ExactIn,
            self.mock_jupiter,
        )
        .await?;
        let buy_route = util::jupiter_route(
            &self.mango_client,
            quote_mint,
            token_mint,
            self.quote_amount,
            slippage,
            JupiterSwapMode::ExactIn,
            self.mock_jupiter,
        )
        .await?;

        let buy_over_oracle = Self::price_over_oracle(quote_per_token_price, buy_route)?;
        let sell_over_oracle = Self::price_over_oracle(token_per_quote_price, sell_route)?;

        log::info!("token {token_index}, buy bps {buy_over_oracle}, sell bps {sell_over_oracle}");

        self.update(
            token_index,
            TokenSwapInfo {
                buy_over_oracle,
                sell_over_oracle,
            },
        );
        Ok(())
    }
}
