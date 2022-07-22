use anchor_lang::prelude::*;

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

    #[account(mut)]
    pub account: UncheckedAccount<'info>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
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
        let mal: MangoAccountLoader<MangoAccount2> =
            MangoAccountLoader::new_init(&ctx.accounts.account)?;
        let account: MangoAccountAcc = mal.load()?;
        require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
        require!(
            account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
            MangoError::SomeError
        );

        require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);

        // Validate open_orders
        require!(
            account
                .serum3_find(serum_market.market_index)
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
        OpenOrdersSlim::from_oo(&open_orders)
    };
    let order = serum_dex::instruction::CancelOrderInstructionV2 {
        side: u8::try_from(side).unwrap().try_into().unwrap(),
        order_id,
    };
    cpi_cancel_order(ctx.accounts, order)?;

    {
        let open_orders = load_open_orders_ref(ctx.accounts.open_orders.as_ref())?;
        let after_oo = OpenOrdersSlim::from_oo(&open_orders);
        let mut mal: MangoAccountLoader<MangoAccount2> =
            MangoAccountLoader::new_init(&ctx.accounts.account)?;
        let mut account: MangoAccountAccMut = mal.load_mut()?;
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
    account: &mut MangoAccountAccMut,
    before_oo: &OpenOrdersSlim,
    after_oo: &OpenOrdersSlim,
) {
    let serum3_account = account.serum3_find_mut(market_index).unwrap();

    if after_oo.native_coin_free > before_oo.native_coin_free {
        let native_coin_free_increase = after_oo.native_coin_free - before_oo.native_coin_free;
        serum3_account.previous_native_coin_reserved =
            cm!(serum3_account.previous_native_coin_reserved - native_coin_free_increase);
    }

    // pc
    if after_oo.native_pc_free > before_oo.native_pc_free {
        let free_pc_increase = after_oo.native_pc_free - before_oo.native_pc_free;
        serum3_account.previous_native_pc_reserved =
            cm!(serum3_account.previous_native_pc_reserved - free_pc_increase);
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
