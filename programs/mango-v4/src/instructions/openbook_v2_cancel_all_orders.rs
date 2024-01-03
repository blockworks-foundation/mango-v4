use anchor_lang::prelude::*;
use openbook_v2::cpi::accounts::CancelOrder;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn openbook_v2_cancel_all_orders(ctx: Context<OpenbookV2CancelOrder>, limit: u8) -> Result<()> {
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load_full()?;
        // account constraint #1
        require!(
            account.fixed.is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );

        let openbook_market = ctx.accounts.openbook_v2_market.load()?;

        // Validate open_orders #2
        require!(
            account
                .openbook_v2_orders(openbook_market.market_index)?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );
    }

    //
    // Cancel
    //
    let account = ctx.accounts.account.load()?;
    let account_seeds = mango_account_seeds!(account);
    cpi_cancel_all_orders(ctx.accounts, &[account_seeds], limit)?;

    // let openbook_market = ctx.accounts.openbook_v2_market.load()?;
    // let oo_ai = &ctx.accounts.open_orders.as_ref();
    // let open_orders = load_open_orders_ref(oo_ai)?;
    // let after_oo = OpenOrdersSlim::from_oo(&open_orders);
    // emit!(Serum3OpenOrdersBalanceLogV2 {
    //     mango_group: ctx.accounts.group.key(),
    //     mango_account: ctx.accounts.account.key(),
    //     market_index: serum_market.market_index,
    //     base_token_index: serum_market.base_token_index,
    //     quote_token_index: serum_market.quote_token_index,
    //     base_total: after_oo.native_base_total(),
    //     base_free: after_oo.native_base_free(),
    //     quote_total: after_oo.native_quote_total(),
    //     quote_free: after_oo.native_quote_free(),
    //     referrer_rebates_accrued: after_oo.native_rebates(),
    // });

    Ok(())
}

fn cpi_cancel_all_orders(ctx: &OpenbookV2CancelOrder, seeds: &[&[&[u8]]], limit: u8) -> Result<()> {
    let cpi_accounts = CancelOrder {
        signer: ctx.account.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        bids: ctx.bids.to_account_info(),
        asks: ctx.asks.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    // todo-pan: maybe allow passing side for cu opt
    openbook_v2::cpi::cancel_all_orders(cpi_ctx, None, limit)
}
