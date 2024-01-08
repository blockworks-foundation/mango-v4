use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;
use crate::util::fill_from_str;

use crate::accounts_ix::*;
use crate::logs::OpenbookV2RegisterMarketLog;

pub fn openbook_v2_register_market(
    ctx: Context<OpenbookV2RegisterMarket>,
    market_index: OpenbookV2MarketIndex,
    name: String,
    oracle_price_band: f32,
) -> Result<()> {
    // TODO: must guard against accidentally using the same market_index twice!

    let base_bank = ctx.accounts.base_bank.load()?;
    let quote_bank = ctx.accounts.quote_bank.load()?;
    let market_external = ctx.accounts.openbook_v2_market_external.load()?;
    require_eq!(
        market_external.quote_mint,
        quote_bank.mint,
        MangoError::SomeError
    );
    require_eq!(
        market_external.base_mint,
        base_bank.mint,
        MangoError::SomeError
    );

    let mut serum_market = ctx.accounts.openbook_v2_market.load_init()?;
    *serum_market = OpenbookV2Market {
        group: ctx.accounts.group.key(),
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        reduce_only: 0,
        force_close: 0,
        padding1: Default::default(),
        name: fill_from_str(&name)?,
        openbook_v2_program: ctx.accounts.openbook_v2_program.key(),
        openbook_v2_market_external: ctx.accounts.openbook_v2_market_external.key(),
        market_index,
        bump: *ctx
            .bumps
            .get("openbook_v2_market")
            .ok_or(MangoError::SomeError)?,
        padding2: Default::default(),
        oracle_price_band,
        registration_time: Clock::get()?.unix_timestamp.try_into().unwrap(),
        reserved: [0; 512],
    };

    let mut openbook_index_reservation = ctx.accounts.index_reservation.load_init()?;
    *openbook_index_reservation = OpenbookV2MarketIndexReservation {
        group: ctx.accounts.group.key(),
        market_index,
        reserved: [0; 38],
    };

    emit!(OpenbookV2RegisterMarketLog {
        mango_group: ctx.accounts.group.key(),
        openbook_market: ctx.accounts.openbook_v2_market.key(),
        market_index,
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        openbook_program: ctx.accounts.openbook_v2_program.key(),
        openbook_market_external: ctx.accounts.openbook_v2_market_external.key(),
    });

    Ok(())
}
