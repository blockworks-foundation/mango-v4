use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct CreatePerpMarket<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub oracle: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [group.key().as_ref(), b"PerpMarket".as_ref(), oracle.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<PerpMarket>(), 
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(
        init,
        seeds = [group.key().as_ref(), b"Asks".as_ref(), perp_market.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<PerpMarket>(),
    )]
    pub asks: AccountLoader<'info, crate::state::Book>,
    #[account(
        init,
        seeds = [group.key().as_ref(), b"Bids".as_ref(), perp_market.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<PerpMarket>(),
    )]
    pub bids: AccountLoader<'info, Book>,
    #[account(
        init,
        seeds = [group.key().as_ref(), b"EventQueue".as_ref(), perp_market.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<PerpMarket>(),
    )]
    pub event_queue: AccountLoader<'info, crate::state::EventQueue>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_perp_market(
    ctx: Context<CreatePerpMarket>,
    quote_lot_size: i64,
    base_lot_size: i64,
) -> Result<()> {
    let mut perp_market = ctx.accounts.perp_market.load_init()?;
    *perp_market = PerpMarket {
        group: ctx.accounts.group.key(),
        oracle: ctx.accounts.oracle.key(),
        bids: ctx.accounts.bids.key(),
        asks: ctx.accounts.asks.key(),
        event_queue: ctx.accounts.event_queue.key(),
        quote_lot_size: quote_lot_size,
        base_lot_size: base_lot_size,
        // long_funding,
        // short_funding,
        // last_updated,
        // open_interest,
        seq_num: 0,
        // fees_accrued,
        // liquidity_mining_info,
        // mngo_vault: ctx.accounts.mngo_vault.key(),
        bump: *ctx.bumps.get("perp_market").ok_or(MangoError::SomeError)?,
    };

    Ok(())
}
