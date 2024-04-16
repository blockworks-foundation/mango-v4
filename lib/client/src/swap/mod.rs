pub mod jupiter_v6;
pub mod sanctum;
pub mod sanctum_state;

use anchor_lang::prelude::*;
use std::str::FromStr;

use crate::{MangoClient, TransactionBuilder};
use fixed::types::I80F48;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Mock,
    V6,
    Sanctum,
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum RawQuote {
    Mock,
    V6(jupiter_v6::QuoteResponse),
    Sanctum(sanctum::QuoteResponse),
}

#[derive(Clone)]
pub struct Quote {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub price_impact_pct: f64,
    pub in_amount: u64,
    pub out_amount: u64,
    pub raw: RawQuote,
}

impl Quote {
    pub fn try_from_v6(query: jupiter_v6::QuoteResponse) -> anyhow::Result<Self> {
        Ok(Quote {
            input_mint: Pubkey::from_str(&query.input_mint)?,
            output_mint: Pubkey::from_str(&query.output_mint)?,
            price_impact_pct: query.price_impact_pct.parse()?,
            in_amount: query
                .in_amount
                .as_ref()
                .map(|a| a.parse())
                .unwrap_or(Ok(0))?,
            out_amount: query.out_amount.parse()?,
            raw: RawQuote::V6(query),
        })
    }

    pub fn try_from_sanctum(
        input_mint: Pubkey,
        output_mint: Pubkey,
        query: sanctum::QuoteResponse,
    ) -> anyhow::Result<Self> {
        Ok(Quote {
            input_mint: input_mint,
            output_mint: output_mint,
            price_impact_pct: query.fee_pct.parse()?,
            in_amount: query
                .in_amount
                .as_ref()
                .map(|a| a.parse())
                .unwrap_or(Ok(0))?,
            out_amount: query.out_amount.parse()?,
            raw: RawQuote::Sanctum(query),
        })
    }

    pub fn first_route_label(&self) -> String {
        let label_maybe = match &self.raw {
            RawQuote::Mock => Some("mock".into()),
            RawQuote::V6(raw) => raw
                .route_plan
                .first()
                .and_then(|v| v.swap_info.as_ref())
                .and_then(|v| v.label.as_ref())
                .cloned(),
            RawQuote::Sanctum(raw) => Some(raw.swap_src.clone()),
        };
        label_maybe.unwrap_or_else(|| "unknown".into())
    }
}

pub struct Swap<'a> {
    pub mango_client: &'a MangoClient,
}

impl<'a> Swap<'a> {
    async fn quote_mock(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
    ) -> anyhow::Result<Quote> {
        let input_token_index = self
            .mango_client
            .context
            .token_by_mint(&input_mint)?
            .token_index;
        let output_token_index = self
            .mango_client
            .context
            .token_by_mint(&output_mint)?
            .token_index;
        let input_price = self
            .mango_client
            .bank_oracle_price(input_token_index)
            .await?;
        let output_price = self
            .mango_client
            .bank_oracle_price(output_token_index)
            .await?;
        let in_amount = amount;
        let out_amount = (I80F48::from(amount) * input_price / output_price).to_num::<u64>();
        Ok(Quote {
            input_mint,
            output_mint,
            price_impact_pct: 0.0,
            in_amount,
            out_amount,
            raw: RawQuote::Mock,
        })
    }

    pub async fn quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        slippage_bps: u64,
        only_direct_routes: bool,
        version: Version,
    ) -> anyhow::Result<Quote> {
        Ok(match version {
            Version::Mock => self.quote_mock(input_mint, output_mint, amount).await?,
            Version::V6 => Quote::try_from_v6(
                self.mango_client
                    .jupiter_v6()
                    .quote(
                        input_mint,
                        output_mint,
                        amount,
                        slippage_bps,
                        only_direct_routes,
                    )
                    .await?,
            )?,
            Version::Sanctum => Quote::try_from_sanctum(
                input_mint,
                output_mint,
                self.mango_client
                    .sanctum()
                    .quote(input_mint, output_mint, amount)
                    .await?,
            )?,
        })
    }

    pub async fn prepare_swap_transaction(
        &self,
        quote: &Quote,
    ) -> anyhow::Result<TransactionBuilder> {
        match &quote.raw {
            RawQuote::Mock => anyhow::bail!("can't prepare jupiter swap for the mock"),
            RawQuote::V6(raw) => {
                self.mango_client
                    .jupiter_v6()
                    .prepare_swap_transaction(raw)
                    .await
            }
            RawQuote::Sanctum(raw) => {
                let max_slippage_bps = (quote.price_impact_pct * 100.0).ceil() as u64;
                self.mango_client
                    .sanctum()
                    .prepare_swap_transaction(
                        quote.input_mint,
                        quote.output_mint,
                        max_slippage_bps,
                        raw,
                    )
                    .await
            }
        }
    }
}
