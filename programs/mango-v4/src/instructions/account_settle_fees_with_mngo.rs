use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::*;

use crate::accounts_ix::*;

pub fn account_settle_fees_with_mngo(
    ctx: Context<AccountSettleFeesWithMngo>,
    max_settle: u64,
) -> Result<()> {
    // Cannot settle with yourself
    require_keys_neq!(
        ctx.accounts.account.key(),
        ctx.accounts.dao_account.key(),
        MangoError::CannotSettleWithSelf
    );

    let group = ctx.accounts.group.load()?;

    let mut mngo_bank = ctx.accounts.mngo_bank.load_mut()?;
    let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;

    let mut account = ctx.accounts.account.load_full_mut()?;
    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );
    let mut dao_account = ctx.accounts.dao_account.load_full_mut()?;

    let bonus_rate = I80F48::from_num(group.fees_mngo_bonus_factor);
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut max_settle =
        I80F48::from_num::<u64>(max_settle.min(account.fixed.discount_settleable_fees_accrued));
    if max_settle == I80F48::ZERO {
        msg!(
            "nothing to settle, (discount_settleable_fees_accrued {})",
            account.fixed.discount_settleable_fees_accrued
        );
        return Ok(());
    }

    // if mngo token position has borrows, skip settling
    let account_mngo_token_position = account.ensure_token_position(mngo_bank.token_index)?.0;
    let account_mngo_native = account_mngo_token_position.native(&mngo_bank);
    if account_mngo_native.is_negative() {
        msg!(
            "mngo token position ({} native mngo) has borrows, nothing will be settled",
            account_mngo_native
        );
        return Ok(());
    }
    let mngo_oracle_price = mngo_bank.oracle_price(
        &AccountInfoRef::borrow(&ctx.accounts.mngo_oracle.as_ref())?,
        Some(Clock::get()?.slot),
    )?;
    let mngo_settle_price = cm!(mngo_oracle_price * bonus_rate);
    // mngo is exchanged at a discount
    let mut max_settle_mngo = cm!(max_settle / mngo_settle_price);
    // settlement is restricted to accounts token position
    max_settle_mngo = max_settle_mngo.min(account_mngo_native);
    max_settle = cm!(max_settle_mngo * mngo_settle_price);

    // move mngo from user to dao
    let dao_mngo_token_position = dao_account.ensure_token_position(mngo_bank.token_index)?.0;
    mngo_bank.withdraw_without_fee(
        account_mngo_token_position,
        max_settle_mngo,
        now_ts,
        mngo_oracle_price,
    )?;
    mngo_bank.deposit(dao_mngo_token_position, max_settle_mngo, now_ts)?;

    // move settlement tokens from dao to user
    let account_settle_token_position = account.ensure_token_position(settle_bank.token_index)?.0;
    let dao_settle_token_position = dao_account
        .ensure_token_position(settle_bank.token_index)?
        .0;
    settle_bank.withdraw_without_fee(
        dao_settle_token_position,
        max_settle,
        now_ts,
        mngo_oracle_price,
    )?;
    settle_bank.deposit(account_settle_token_position, max_settle, now_ts)?;

    account.fixed.discount_settleable_fees_accrued = account
        .fixed
        .discount_settleable_fees_accrued
        .saturating_sub(max_settle.ceil().to_num::<u64>());
    msg!(
        "settled {} native fees with {} native mngo",
        max_settle,
        max_settle_mngo
    );

    // Ensure dao mango account has no liabilities after we do the token swap
    for ele in dao_account.all_token_positions() {
        require!(ele.indexed_position.is_positive(), MangoError::SomeError);
    }
    require_eq!(
        dao_account.all_perp_positions().count(),
        0,
        MangoError::SomeError
    );

    Ok(())
}
