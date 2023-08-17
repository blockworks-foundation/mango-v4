pub mod v4;
pub mod v6;

use anchor_lang::prelude::*;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Mock,
    V4,
    V6,
}

#[derive(Clone)]
pub enum RawQuote {
    Mock,
    V4(v4::QueryRoute),
    V6(v6::QuoteResponse),
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
    pub fn try_from_v4(
        input_mint: Pubkey,
        output_mint: Pubkey,
        route: v4::QueryRoute,
    ) -> anyhow::Result<Self> {
        Ok(Quote {
            input_mint,
            output_mint,
            price_impact_pct: route.price_impact_pct,
            in_amount: route.in_amount.parse()?,
            out_amount: route.out_amount.parse()?,
            raw: RawQuote::V4(route),
        })
    }

    pub fn try_from_v6(query: v6::QuoteResponse) -> anyhow::Result<Self> {
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

    pub fn first_route_label(&self) -> String {
        let label_maybe = match &self.raw {
            RawQuote::Mock => Some("mock".into()),
            RawQuote::V4(raw) => raw.market_infos.first().map(|v| v.label.clone()),
            RawQuote::V6(raw) => raw
                .route_plan
                .first()
                .and_then(|v| v.swap_info.as_ref())
                .and_then(|v| v.label.as_ref())
                .cloned(),
        };
        label_maybe.unwrap_or("unknown".into())
    }
}
