use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::serum3_cpi::{load_market_state, pubkey_from_u64_array};
use crate::state::*;
use crate::util::fill_from_str;

use crate::accounts_ix::*;
use crate::logs::Serum3RegisterMarketLog;

pub fn serum3_register_market(
    ctx: Context<Serum3RegisterMarket>,
    market_index: Serum3MarketIndex,
    name: String,
) -> Result<()> {
    // TODO: must guard against accidentally using the same market_index twice!

    let base_bank = ctx.accounts.base_bank.load()?;
    let quote_bank = ctx.accounts.quote_bank.load()?;
    let market_external = load_market_state(
        &ctx.accounts.serum_market_external,
        &ctx.accounts.serum_program.key(),
    )?;
    require!(
        pubkey_from_u64_array(market_external.pc_mint) == quote_bank.mint,
        MangoError::SomeError
    );
    require!(
        pubkey_from_u64_array(market_external.coin_mint) == base_bank.mint,
        MangoError::SomeError
    );

    let mut serum_market = ctx.accounts.serum_market.load_init()?;
    *serum_market = Serum3Market {
        group: ctx.accounts.group.key(),
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        reduce_only: 0,
        padding1: Default::default(),
        name: fill_from_str(&name)?,
        serum_program: ctx.accounts.serum_program.key(),
        serum_market_external: ctx.accounts.serum_market_external.key(),
        market_index,
        bump: *ctx.bumps.get("serum_market").ok_or(MangoError::SomeError)?,
        padding2: Default::default(),
        registration_time: Clock::get()?.unix_timestamp.try_into().unwrap(),
        reserved: [0; 128],
    };

    let mut serum_index_reservation = ctx.accounts.index_reservation.load_init()?;
    *serum_index_reservation = Serum3MarketIndexReservation {
        group: ctx.accounts.group.key(),
        market_index,
        reserved: [0; 38],
    };

    emit!(Serum3RegisterMarketLog {
        mango_group: ctx.accounts.group.key(),
        serum_market: ctx.accounts.serum_market.key(),
        market_index,
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        serum_program: ctx.accounts.serum_program.key(),
        serum_program_external: ctx.accounts.serum_market_external.key(),
    });

    Ok(())
}
