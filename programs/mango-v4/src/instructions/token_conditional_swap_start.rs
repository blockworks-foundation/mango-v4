use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::state::*;

/// Incentive to pay to callers who start an auction
// TODO: $0.001 is ok? around 10x tx fee currently
const TCS_START_INCENTIVE: u64 = 1_000; // $0.001

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_start(
    ctx: Context<TokenConditionalSwapStart>,
    token_conditional_swap_index: usize,
    token_conditional_swap_id: u64,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();
    let account_key = ctx.accounts.account.key();
    let caller_key = ctx.accounts.caller.key();
    require_keys_neq!(account_key, caller_key);

    let mut account = ctx.accounts.account.load_full_mut()?;
    require!(
        !account.fixed.being_liquidated(),
        MangoError::BeingLiquidated,
    );

    let mut caller = ctx.accounts.caller.load_full_mut()?;

    let mut account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
        .context("create account retriever")?;

    let tcs = account.token_conditional_swap_by_index(token_conditional_swap_index)?;
    require!(tcs.has_data(), MangoError::SomeError);
    require_eq!(tcs.id, token_conditional_swap_id);
    let buy_token_index = tcs.buy_token_index;
    let sell_token_index = tcs.sell_token_index;
    let remaining_sell = tcs.remaining_sell();
    let allow_borrows = tcs.allow_creating_borrows();
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    require!(tcs.has_incentive_for_starting(), MangoError::SomeError);

    let mut health_cache = new_health_cache(&account.borrow(), &account_retriever)
        .context("create liqee health cache")?;
    let pre_init_health = account.check_health_pre(&health_cache)?;

    let (sell_bank, sell_oracle_price, buy_bank_and_oracle_opt) =
        account_retriever.banks_mut_and_oracles(sell_token_index, buy_token_index)?;
    let (_, buy_oracle_price) = buy_bank_and_oracle_opt.unwrap();

    //
    // Check the tcs price condition
    //
    let price = buy_oracle_price.to_num::<f64>() / sell_oracle_price.to_num::<f64>();
    require!(tcs.is_startable(price, now_ts), MangoError::SomeError);

    //
    // Transfer the starting incentive
    //

    // We allow the incentive to be < 1 native token because of tokens like BTC, where 1 native token
    // far exceeds the incentive value.
    let incentive =
        (I80F48::from(TCS_START_INCENTIVE) / sell_oracle_price).min(I80F48::from(remaining_sell));
    // However, the tcs tracking is in u64 units. We need to live with the fact of
    // not accounting the incentive fee perfectly.
    let incentive_native = incentive.clamp_to_u64();

    let (account_sell_token, account_sell_raw_index) =
        account.token_position_mut(sell_token_index)?;
    let (caller_sell_token, caller_sell_raw_index, _) =
        caller.ensure_token_position(sell_token_index)?;

    sell_bank.deposit(caller_sell_token, I80F48::from(incentive), now_ts)?;

    // This withdraw might be a borrow, so can fail due to net borrows or reduce-only
    let account_sell_pre_balance = account_sell_token.native(sell_bank);
    sell_bank.withdraw_with_fee(account_sell_token, I80F48::from(incentive), now_ts)?;
    let account_sell_post_balance = account_sell_token.native(sell_bank);
    if account_sell_post_balance < 0 {
        require!(allow_borrows, MangoError::SomeError);
        require!(!sell_bank.are_borrows_reduce_only(), MangoError::SomeError);
        sell_bank.check_net_borrows(sell_oracle_price)?;
    }

    health_cache.adjust_token_balance(
        sell_bank,
        account_sell_post_balance - account_sell_pre_balance,
    )?;

    //
    // Start the tcs
    //
    let tcs = account.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;
    tcs.start_timestamp = now_ts;
    tcs.sold += incentive_native;
    assert!(tcs.passed_start(now_ts));

    // TODO: log auction start

    account.check_health_post(&health_cache, pre_init_health)?;

    Ok(())
}
