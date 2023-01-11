use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::serum3_cpi::{load_market_state, pubkey_from_u64_array};
use crate::state::*;
use crate::util::fill_from_str;

use crate::logs::Serum3RegisterMarketLog;

#[derive(Accounts)]
#[instruction(market_index: Serum3MarketIndex)]
pub struct Serum3RegisterMarket<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = group.load()?.is_operational(),
        constraint = group.load()?.serum3_supported()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    /// CHECK: Can register a market for any serum program
    pub serum_program: UncheckedAccount<'info>,
    /// CHECK: Can register any serum market
    pub serum_market_external: UncheckedAccount<'info>,

    #[account(
        init,
        // using the serum_market_external in the seed guards against registering the same market twice
        seeds = [b"Serum3Market".as_ref(), group.key().as_ref(), serum_market_external.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Serum3Market>(),
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    #[account(
        init,
        // block using the same market index twice
        seeds = [b"Serum3Index".as_ref(), group.key().as_ref(), &market_index.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Serum3MarketIndexReservation>(),
    )]
    pub index_reservation: AccountLoader<'info, Serum3MarketIndexReservation>,

    #[account(has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

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
        market_index: market_index,
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        serum_program: ctx.accounts.serum_program.key(),
        serum_program_external: ctx.accounts.serum_market_external.key(),
    });

    Ok(())
}
