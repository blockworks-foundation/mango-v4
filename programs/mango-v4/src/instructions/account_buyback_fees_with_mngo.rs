use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::*;

use crate::accounts_ix::*;

pub fn account_buyback_fees_with_mngo(
    ctx: Context<AccountBuybackFeesWithMngo>,
    max_buyback: u64,
) -> Result<()> {
    // Cannot buyback from yourself
    require_keys_neq!(
        ctx.accounts.account.key(),
        ctx.accounts.dao_account.key(),
        MangoError::SomeError
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let mut dao_account = ctx.accounts.dao_account.load_full_mut()?;

    let group = ctx.accounts.group.load()?;

    let mut mngo_bank = ctx.accounts.mngo_bank.load_mut()?;
    let mut fees_bank = ctx.accounts.fees_bank.load_mut()?;

    let bonus_rate = I80F48::from_num(group.fees_mngo_bonus_factor);
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();

    // quick return if nothing to buyback
    let mut max_buyback =
        I80F48::from_num::<u64>(max_buyback.min(account.fixed.discount_buyback_fees_accrued));
    if max_buyback == I80F48::ZERO {
        msg!(
            "nothing to buyback, (discount_buyback_fees_accrued {})",
            account.fixed.discount_buyback_fees_accrued
        );
        return Ok(());
    }

    // if mngo token position has borrows, skip buyback
    let (account_mngo_token_position, account_mngo_raw_token_index, _) =
        account.ensure_token_position(mngo_bank.token_index)?;
    let account_mngo_native = account_mngo_token_position.native(&mngo_bank);
    if account_mngo_native.is_negative() {
        msg!(
            "account mngo token position ({} native mngo) has borrows, nothing will be bought back",
            account_mngo_native
        );
        return Ok(());
    }

    // compute max mngo to swap for fees
    let mngo_oracle_price = mngo_bank.oracle_price(
        &AccountInfoRef::borrow(&ctx.accounts.mngo_oracle.as_ref())?,
        Some(Clock::get()?.slot),
    )?;
    let mngo_buyback_price = cm!(mngo_oracle_price * bonus_rate);
    // mngo is exchanged at a discount
    let mut max_buyback_mngo = cm!(max_buyback / mngo_buyback_price);
    // buyback is restricted to accounts token position
    max_buyback_mngo = max_buyback_mngo.min(account_mngo_native);
    max_buyback = cm!(max_buyback_mngo * mngo_buyback_price);

    // move mngo from user to dao
    let (dao_mngo_token_position, dao_mngo_raw_token_index, _) =
        dao_account.ensure_token_position(mngo_bank.token_index)?;
    let in_use = mngo_bank.withdraw_without_fee(
        account_mngo_token_position,
        max_buyback_mngo,
        now_ts,
        mngo_oracle_price,
    )?;
    if !in_use {
        account.deactivate_token_position_and_log(
            account_mngo_raw_token_index,
            ctx.accounts.account.key(),
        );
    }
    let in_use = mngo_bank.deposit(dao_mngo_token_position, max_buyback_mngo, now_ts)?;
    if !in_use {
        dao_account.deactivate_token_position_and_log(
            dao_mngo_raw_token_index,
            ctx.accounts.dao_account.key(),
        );
    }

    // move fees from dao to user
    let (account_fees_token_position, account_fees_raw_token_index, _) =
        account.ensure_token_position(fees_bank.token_index)?;
    let (dao_fees_token_position, dao_fees_raw_token_index, _) =
        dao_account.ensure_token_position(fees_bank.token_index)?;
    let dao_fees_native = dao_fees_token_position.native(&fees_bank);
    if dao_fees_native.is_negative() || dao_fees_native < max_buyback {
        msg!(
            "dao fees token position ({} native fees) is lesser than max buyback {}, nothing will be bought back",
            dao_fees_native, max_buyback
        );
        return Ok(());
    }
    let in_use = fees_bank.withdraw_without_fee(
        dao_fees_token_position,
        max_buyback,
        now_ts,
        mngo_oracle_price,
    )?;
    if !in_use {
        dao_account.deactivate_token_position_and_log(
            dao_fees_raw_token_index,
            ctx.accounts.dao_account.key(),
        );
    }
    let in_use = fees_bank.deposit(account_fees_token_position, max_buyback, now_ts)?;
    if !in_use {
        account.deactivate_token_position_and_log(
            account_fees_raw_token_index,
            ctx.accounts.account.key(),
        );
    }

    account.fixed.discount_buyback_fees_accrued = account
        .fixed
        .discount_buyback_fees_accrued
        .saturating_sub(max_buyback.ceil().to_num::<u64>());
    msg!(
        "bought back {} native fees with {} native mngo",
        max_buyback,
        max_buyback_mngo
    );

    // ensure dao mango account has no liabilities after we do the token swap
    for ele in dao_account.all_token_positions() {
        require!(!ele.indexed_position.is_negative(), MangoError::SomeError);
    }
    require_eq!(
        dao_account.active_perp_positions().count(),
        0,
        MangoError::SomeError
    );

    Ok(())
}
