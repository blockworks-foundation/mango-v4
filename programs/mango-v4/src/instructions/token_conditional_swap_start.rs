use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::logs::{emit_stack, TokenBalanceLog, TokenConditionalSwapStartLog};
use crate::state::*;

#[allow(clippy::too_many_arguments)]
pub fn token_conditional_swap_start(
    ctx: Context<TokenConditionalSwapStart>,
    token_conditional_swap_index: usize,
    token_conditional_swap_id: u64,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();
    let liqee_key = ctx.accounts.liqee.key();
    let liqor_key = ctx.accounts.liqor.key();

    let mut liqee = ctx.accounts.liqee.load_full_mut()?;
    require!(!liqee.fixed.being_liquidated(), MangoError::BeingLiquidated,);

    let mut liqor = ctx.accounts.liqor.load_full_mut()?;

    let mut account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
        .context("create account retriever")?;

    let tcs = liqee
        .token_conditional_swap_by_index(token_conditional_swap_index)?
        .clone();
    require!(tcs.is_configured(), MangoError::TokenConditionalSwapNotSet);
    require_eq!(
        tcs.id,
        token_conditional_swap_id,
        MangoError::TokenConditionalSwapIndexIdMismatch
    );
    let buy_token_index = tcs.buy_token_index;
    let sell_token_index = tcs.sell_token_index;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    require!(
        tcs.is_startable_type(),
        MangoError::TokenConditionalSwapTypeNotStartable
    );

    let mut health_cache = new_health_cache(&liqee.borrow(), &account_retriever, now_ts)
        .context("create liqee health cache")?;
    let pre_init_health = liqee.check_health_pre(&health_cache)?;

    let (sell_bank, sell_oracle_price, buy_bank_and_oracle_opt) =
        account_retriever.banks_mut_and_oracles(sell_token_index, buy_token_index)?;
    let (_, buy_oracle_price) = buy_bank_and_oracle_opt.unwrap();

    //
    // Check the tcs price condition
    //
    let price = buy_oracle_price.to_num::<f64>() / sell_oracle_price.to_num::<f64>();
    tcs.check_startable(price, now_ts)?;

    //
    // Transfer the starting incentive
    //

    // We allow the incentive to be < 1 native token because of tokens like BTC, where 1 native token
    // far exceeds the incentive value.
    let incentive = (I80F48::from(TCS_START_INCENTIVE) / sell_oracle_price)
        .min(I80F48::from(tcs.remaining_sell()))
        // Limiting to remaining deposits is too strict, since this could be a deposit
        // to deposit transfer, but this is good enough to make the incentive deposit
        // guaranteed to not exceed the limit.
        .min(sell_bank.remaining_deposits_until_limit())
        .max(I80F48::ZERO);
    // The tcs tracking is in u64 units. We need to live with the fact of
    // not accounting the incentive fee perfectly.
    let incentive_native = incentive.clamp_to_u64();

    let (liqee_sell_token, liqee_sell_raw_index) = liqee.token_position_mut(sell_token_index)?;
    let (liqor_sell_token, liqor_sell_raw_index, _) =
        liqor.ensure_token_position(sell_token_index)?;

    let liqee_sell_pre_balance = liqee_sell_token.native(sell_bank);
    sell_bank.checked_transfer_with_fee(
        liqee_sell_token,
        incentive,
        liqor_sell_token,
        incentive,
        now_ts,
        sell_oracle_price,
    )?;
    let liqee_sell_post_balance = liqee_sell_token.native(sell_bank);
    if liqee_sell_post_balance < 0 {
        require!(
            tcs.allow_creating_borrows(),
            MangoError::TokenConditionalSwapCantPayIncentive
        );
    }

    health_cache
        .adjust_token_balance(sell_bank, liqee_sell_post_balance - liqee_sell_pre_balance)?;

    emit_stack(TokenBalanceLog {
        mango_group: *group_pk,
        mango_account: liqee_key,
        token_index: sell_token_index,
        indexed_position: liqee_sell_token.indexed_position.to_bits(),
        deposit_index: sell_bank.deposit_index.to_bits(),
        borrow_index: sell_bank.borrow_index.to_bits(),
    });
    emit_stack(TokenBalanceLog {
        mango_group: *group_pk,
        mango_account: liqor_key,
        token_index: sell_token_index,
        indexed_position: liqor_sell_token.indexed_position.to_bits(),
        deposit_index: sell_bank.deposit_index.to_bits(),
        borrow_index: sell_bank.borrow_index.to_bits(),
    });
    emit_stack(TokenConditionalSwapStartLog {
        mango_group: *group_pk,
        mango_account: liqee_key,
        caller: liqor_key,
        token_conditional_swap_id: tcs.id,
        incentive_token_index: sell_token_index,
        incentive_amount: incentive_native,
    });

    //
    // Start the tcs
    //
    let tcs = liqee.token_conditional_swap_mut_by_index(token_conditional_swap_index)?;
    tcs.start_timestamp = now_ts;
    assert!(tcs.passed_start(now_ts));

    tcs.sold += incentive_native;
    assert!(tcs.sold <= tcs.max_sell);

    liqee.check_health_post(&health_cache, pre_init_health)?;

    Ok(())
}
