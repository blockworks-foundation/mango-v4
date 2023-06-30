use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::logs::TokenConditionalSwapCreateLog;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_create(
    ctx: Context<TokenConditionalSwapCreate>,
    token_conditional_swap: TokenConditionalSwap,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    if token_conditional_swap.is_expired(now_ts) {
        msg!("Already expired, ignoring");
        return Ok(());
    }

    let mut account = ctx.accounts.account.load_full_mut()?;

    let id = account.fixed.next_token_conditional_swap_id;
    account.fixed.next_token_conditional_swap_id =
        account.fixed.next_token_conditional_swap_id.wrapping_add(1);

    let tcs = account.free_token_conditional_swap_mut()?;
    *tcs = token_conditional_swap;
    tcs.id = id;
    tcs.taker_fee_bps = group.token_conditional_swap_taker_fee_bps;
    tcs.maker_fee_bps = group.token_conditional_swap_maker_fee_bps;
    tcs.has_data = 1;
    tcs.bought = 0;
    tcs.sold = 0;

    require_neq!(tcs.buy_token_index, tcs.sell_token_index);
    require_gte!(tcs.price_premium_bps, 0);
    require_gte!(tcs.maker_fee_bps, 0);
    require_gte!(tcs.taker_fee_bps, 0);
    require_gte!(tcs.price_lower_limit, 0.0);
    require_gte!(tcs.price_upper_limit, 0.0);

    emit!(TokenConditionalSwapCreateLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        id,
        max_buy: tcs.max_buy,
        max_sell: tcs.max_sell,
        expiry_timestamp: tcs.expiry_timestamp,
        price_lower_limit: tcs.price_lower_limit,
        price_upper_limit: tcs.price_upper_limit,
        price_premium_bps: tcs.price_premium_bps,
        taker_fee_bps: tcs.taker_fee_bps,
        maker_fee_bps: tcs.maker_fee_bps,
        buy_token_index: tcs.buy_token_index,
        sell_token_index: tcs.sell_token_index,
        allow_creating_borrows: tcs.allow_creating_borrows(),
        allow_creating_deposits: tcs.allow_creating_deposits(),
    });

    Ok(())
}
