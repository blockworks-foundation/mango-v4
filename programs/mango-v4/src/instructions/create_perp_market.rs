use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
#[instruction(perp_market_index: PerpMarketIndex)]
pub struct CreatePerpMarket<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub oracle: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"PerpMarket".as_ref(), &perp_market_index.to_le_bytes().as_ref()],
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
        space = 8 + std::mem::size_of::<Book>(),
    )]
    pub asks: AccountLoader<'info, crate::state::Book>,
    #[account(
        init,
        seeds = [group.key().as_ref(), b"Bids".as_ref(), perp_market.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Book>(),
    )]
    pub bids: AccountLoader<'info, Book>,
    #[account(
        init,
        seeds = [group.key().as_ref(), b"EventQueue".as_ref(), perp_market.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<EventQueue>(),
    )]
    pub event_queue: AccountLoader<'info, crate::state::EventQueue>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_perp_market(
    ctx: Context<CreatePerpMarket>,
    perp_market_index: PerpMarketIndex,
    base_token_index_opt: Option<TokenIndex>,
    quote_token_index: TokenIndex,
    quote_lot_size: i64,
    base_lot_size: i64,
    // todo
    // base token index (optional)
    // quote token index
    // oracle
    // perp market index
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
        perp_market_index,
        base_token_index: base_token_index_opt.ok_or(TokenIndex::MAX).unwrap(),
        quote_token_index,
    };

    let mut asks = ctx.accounts.asks.load_init()?;
    *asks = Book {};

    let mut bids = ctx.accounts.bids.load_init()?;
    *bids = Book {};

    let mut event_queue = ctx.accounts.event_queue.load_init()?;
    *event_queue = EventQueue {};

    Ok(())
}
