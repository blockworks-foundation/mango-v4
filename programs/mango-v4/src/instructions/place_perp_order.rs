use anchor_lang::prelude::*;

use crate::state::{
    oracle_price, Book, BookSide, Group, MangoAccount, OrderType, PerpMarket, Side,
};

#[derive(Accounts)]
pub struct PlacePerpOrder<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,
    pub oracle: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

pub fn place_perp_order(
    ctx: Context<PlacePerpOrder>,
    // TODO side is harcoded for now
    // maybe new_bid and new_ask can be folded into one function
    // side: Side,
    price: i64,
    max_base_quantity: i64,
    max_quote_quantity: i64,
    client_order_id: u64,
    order_type: OrderType,
    // TODO reduce_only relies on event queue
    // reduce_only: bool,
    expiry_timestamp: u64,
    limit: u8,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_mut()?;
    let mango_account_pk = ctx.accounts.account.key();

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let bids = &ctx.accounts.bids.to_account_info();
    let asks = &ctx.accounts.asks.to_account_info();
    let mut book = Book::load_checked(&bids, &asks, &perp_market)?;

    let oracle_price = oracle_price(&ctx.accounts.oracle.to_account_info())?;

    let now_ts = Clock::get()?.unix_timestamp as u64;
    let time_in_force = if expiry_timestamp != 0 {
        // If expiry is far in the future, clamp to 255 seconds
        let tif = expiry_timestamp.saturating_sub(now_ts).min(255);
        if tif == 0 {
            // If expiry is in the past, ignore the order
            msg!("Order is already expired");
            return Ok(());
        }
        tif as u8
    } else {
        // Never expire
        0
    };

    // TODO reduce_only based on event queue

    book.new_bid(
        // program_id: &Pubkey,
        // mango_group: &MangoGroup,
        // mango_group_pk: &Pubkey,
        // mango_cache: &MangoCache,
        // event_queue: &mut EventQueue,
        &mut perp_market,
        oracle_price,
        &mut account,
        &mango_account_pk,
        // market_index: usize,
        price,
        max_base_quantity,
        max_quote_quantity,
        order_type,
        time_in_force,
        client_order_id,
        now_ts,
        // referrer_mango_account_ai: Option<&AccountInfo>,
        limit,
    )?;

    Ok(())
}
