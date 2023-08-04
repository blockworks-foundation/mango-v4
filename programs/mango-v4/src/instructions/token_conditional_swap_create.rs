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
    {
        let buy_pos = account
            .ensure_token_position(token_conditional_swap.buy_token_index)?
            .0;
        buy_pos.increment_in_use();
        let sell_pos = account
            .ensure_token_position(token_conditional_swap.sell_token_index)?
            .0;
        sell_pos.increment_in_use();
    }

    let id = account.fixed.next_token_conditional_swap_id;
    account.fixed.next_token_conditional_swap_id =
        account.fixed.next_token_conditional_swap_id.wrapping_add(1);

    let buy_bank = ctx.accounts.buy_bank.load()?;
    let sell_bank = ctx.accounts.sell_bank.load()?;

    let tcs = account.free_token_conditional_swap_mut()?;
    *tcs = token_conditional_swap;
    tcs.id = id;
    tcs.taker_fee_fraction = buy_bank
        .token_conditional_swap_taker_fee_fraction
        .max(sell_bank.token_conditional_swap_taker_fee_fraction);
    tcs.maker_fee_fraction = buy_bank
        .token_conditional_swap_maker_fee_fraction
        .max(sell_bank.token_conditional_swap_maker_fee_fraction);
    tcs.has_data = 1;
    tcs.bought = 0;
    tcs.sold = 0;

    require_neq!(tcs.buy_token_index, tcs.sell_token_index);
    require_gte!(tcs.price_premium_fraction, 0.0);
    require_gte!(tcs.maker_fee_fraction, 0.0);
    require_gte!(tcs.taker_fee_fraction, 0.0);
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
        price_premium_fraction: tcs.price_premium_fraction,
        taker_fee_fraction: tcs.taker_fee_fraction,
        maker_fee_fraction: tcs.maker_fee_fraction,
        buy_token_index: tcs.buy_token_index,
        sell_token_index: tcs.sell_token_index,
        allow_creating_borrows: tcs.allow_creating_borrows(),
        allow_creating_deposits: tcs.allow_creating_deposits(),
    });

    Ok(())
}
