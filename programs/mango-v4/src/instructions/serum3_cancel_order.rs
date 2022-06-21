use anchor_lang::prelude::*;
use fixed::types::I80F48;
use serum_dex::instruction::CancelOrderInstructionV2;

use crate::error::*;
use crate::serum3_cpi::load_open_orders_ref;
use crate::state::*;

use super::OpenOrdersSlim;
use super::Serum3Side;
use checked_math as cm;

#[derive(Accounts)]
pub struct Serum3CancelOrder<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    // Validated inline
    #[account(mut)]
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_event_queue: UncheckedAccount<'info>,
}

pub fn serum3_cancel_order(
    ctx: Context<Serum3CancelOrder>,
    side: Serum3Side,
    order_id: u128,
) -> Result<()> {
    let serum_market = ctx.accounts.serum_market.load()?;

    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
        require!(account.is_bankrupt == 0, MangoError::IsBankrupt);

        // Validate open_orders
        require!(
            account
                .serum3
                .find(serum_market.market_index)
                .ok_or_else(|| error!(MangoError::SomeError))?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );
    }

    //
    // Cancel
    //
    let before_oo = {
        let open_orders = load_open_orders_ref(ctx.accounts.open_orders.as_ref())?;
        OpenOrdersSlim::fromOO(&open_orders)
    };
    let order = serum_dex::instruction::CancelOrderInstructionV2 {
        side: u8::try_from(side).unwrap().try_into().unwrap(),
        order_id,
    };
    cpi_cancel_order(ctx.accounts, order)?;

    {
        let open_orders = load_open_orders_ref(ctx.accounts.open_orders.as_ref())?;
        let after_oo = OpenOrdersSlim::fromOO(&open_orders);
        let mut account = ctx.accounts.account.load_mut()?;
        decrease_maybe_loan(
            serum_market.market_index,
            &mut account,
            &before_oo,
            &after_oo,
        );
    };

    Ok(())
}

// if free has increased, the free increase is reduction in reserved, reduce this from
// the cached
pub fn decrease_maybe_loan(
    market_index: Serum3MarketIndex,
    account: &mut MangoAccount,
    before_oo: &OpenOrdersSlim,
    after_oo: &OpenOrdersSlim,
) {
    let serum3_account = account.serum3.find_mut(market_index).unwrap();

    if after_oo.native_coin_free > before_oo.native_coin_free {
        let native_coin_free_increase = after_oo.native_coin_free - before_oo.native_coin_free;
        serum3_account.native_coin_reserved_cached =
            cm!(serum3_account.native_coin_reserved_cached - native_coin_free_increase);
    }

    // pc
    if after_oo.native_pc_free > before_oo.native_pc_free {
        let free_pc_increase = after_oo.native_pc_free - before_oo.native_pc_free;
        serum3_account.native_pc_reserved_cached =
            cm!(serum3_account.native_pc_reserved_cached - free_pc_increase);
    }
}

fn cpi_cancel_order(ctx: &Serum3CancelOrder, order: CancelOrderInstructionV2) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::CancelOrder {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        bids: ctx.market_bids.to_account_info(),
        asks: ctx.market_asks.to_account_info(),
        event_queue: ctx.market_event_queue.to_account_info(),

        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
    }
    .cancel_one(&group, order)
}
