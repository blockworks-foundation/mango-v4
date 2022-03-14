use anchor_lang::prelude::*;
use anchor_spl::dex;
use anchor_spl::token::{Token, TokenAccount};
use dex::serum_dex;
use serum_dex::matching::Side;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct PlaceSerumOrder<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        //constraint = open_orders in account.spot_open_orders_map
    )]
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, SerumMarket>,

    // TODO: limit?
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub serum_market_external: UncheckedAccount<'info>,

    #[account(mut)]
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_event_queue: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_request_queue: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_quote_vault: UncheckedAccount<'info>,

    // TODO: everything; do we need to pass both, or just payer?
    // these are all potentially mut too, if we settle immediately?
    pub quote_bank: AccountLoader<'info, Bank>,
    pub quote_vault: Account<'info, TokenAccount>,
    pub base_bank: AccountLoader<'info, Bank>,
    pub base_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn place_serum_order(
    ctx: Context<PlaceSerumOrder>,
    order: serum_dex::instruction::NewOrderInstructionV3,
) -> Result<()> {
    let order_payer_token_account = match order.side {
        Side::Ask => ctx.accounts.base_vault.to_account_info(),
        Side::Bid => ctx.accounts.quote_vault.to_account_info(),
    };

    let context = CpiContext::new(
        ctx.accounts.serum_program.to_account_info(),
        dex::NewOrderV3 {
            // generic accounts
            market: ctx.accounts.serum_market_external.to_account_info(),
            request_queue: ctx.accounts.market_request_queue.to_account_info(),
            event_queue: ctx.accounts.market_event_queue.to_account_info(),
            market_bids: ctx.accounts.market_bids.to_account_info(),
            market_asks: ctx.accounts.market_asks.to_account_info(),
            coin_vault: ctx.accounts.market_base_vault.to_account_info(),
            pc_vault: ctx.accounts.market_quote_vault.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),

            // user accounts
            open_orders: ctx.accounts.open_orders.to_account_info(),
            open_orders_authority: ctx.accounts.serum_market.to_account_info(),
            order_payer_token_account,
        },
    );

    let serum_market = ctx.accounts.serum_market.load()?;
    let seeds = serum_market_seeds!(serum_market);
    dex::new_order_v3(
        context.with_signer(&[seeds]),
        order.side,
        order.limit_price,
        order.max_coin_qty,
        order.max_native_pc_qty_including_fees,
        order.self_trade_behavior,
        order.order_type,
        order.client_order_id,
        order.limit,
    )?;

    // Health check
    let account = ctx.accounts.account.load()?;
    let health = compute_health(&account, &ctx.remaining_accounts)?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::SomeError);

    Ok(())
}
