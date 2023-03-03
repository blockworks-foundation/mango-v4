use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::*;

use crate::accounts_ix::*;

use crate::logs::{AccountBuybackFeesWithMngoLog, TokenBalanceLog};

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

    let bonus_factor = I80F48::from_num(group.buyback_fees_mngo_bonus_factor);

    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();
    account
        .fixed
        .expire_buyback_fees(now_ts, group.buyback_fees_expiry_interval);

    // quick return if nothing to buyback
    let mut max_buyback = {
        let dao_fees_token_position = dao_account.ensure_token_position(fees_bank.token_index)?.0;
        let dao_fees_native = dao_fees_token_position.native(&fees_bank);
        I80F48::from_num::<u64>(max_buyback.min(account.fixed.buyback_fees_accrued()))
            .min(dao_fees_native)
    };
    if max_buyback <= I80F48::ZERO {
        msg!(
            "nothing to buyback, (buyback_fees_accrued {})",
            account.fixed.buyback_fees_accrued()
        );
        return Ok(());
    }

    // if mngo token position has borrows, skip buyback
    let account_mngo_native = account
        .token_position(mngo_bank.token_index)
        .map(|tp| tp.native(&mngo_bank))
        .unwrap_or(I80F48::ZERO);
    if account_mngo_native <= I80F48::ZERO {
        msg!(
            "account mngo token position ({} native mngo) is <= 0, nothing will be bought back",
            account_mngo_native
        );
        return Ok(());
    }
    let (account_mngo_token_position, account_mngo_raw_token_index, _) =
        account.ensure_token_position(mngo_bank.token_index)?;

    // compute max mngo to swap for fees
    let mngo_oracle_price = mngo_bank.oracle_price(
        &AccountInfoRef::borrow(&ctx.accounts.mngo_oracle.as_ref())?,
        Some(Clock::get()?.slot),
    )?;
    let mngo_buyback_price = mngo_oracle_price.min(mngo_bank.stable_price()) * bonus_factor;
    // mngo is exchanged at a discount
    let mut max_buyback_mngo = max_buyback / mngo_buyback_price;
    // buyback is restricted to account's token position
    max_buyback_mngo = max_buyback_mngo.min(account_mngo_native);
    max_buyback = max_buyback_mngo * mngo_buyback_price;

    // move mngo from user to dao
    let (dao_mngo_token_position, dao_mngo_raw_token_index, _) =
        dao_account.ensure_token_position(mngo_bank.token_index)?;
    require!(
        dao_mngo_token_position.indexed_position >= I80F48::ZERO,
        MangoError::SomeError
    );
    let in_use = mngo_bank.withdraw_without_fee(
        account_mngo_token_position,
        max_buyback_mngo,
        now_ts,
        mngo_oracle_price,
    )?;
    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index: mngo_bank.token_index,
        indexed_position: account_mngo_token_position.indexed_position.to_bits(),
        deposit_index: mngo_bank.deposit_index.to_bits(),
        borrow_index: mngo_bank.borrow_index.to_bits(),
    });
    if !in_use {
        account.deactivate_token_position_and_log(
            account_mngo_raw_token_index,
            ctx.accounts.account.key(),
        );
    }
    mngo_bank.deposit(dao_mngo_token_position, max_buyback_mngo, now_ts)?;

    // move fees from dao to user
    let (account_fees_token_position, account_fees_raw_token_index, _) =
        account.ensure_token_position(fees_bank.token_index)?;
    let (dao_fees_token_position, dao_fees_raw_token_index, _) =
        dao_account.ensure_token_position(fees_bank.token_index)?;
    let dao_fees_native = dao_fees_token_position.native(&fees_bank);
    assert!(dao_fees_native >= max_buyback);
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
    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index: fees_bank.token_index,
        indexed_position: account_fees_token_position.indexed_position.to_bits(),
        deposit_index: fees_bank.deposit_index.to_bits(),
        borrow_index: fees_bank.borrow_index.to_bits(),
    });
    if !in_use {
        account.deactivate_token_position_and_log(
            account_fees_raw_token_index,
            ctx.accounts.account.key(),
        );
    }

    account
        .fixed
        .reduce_buyback_fees_accrued(max_buyback.ceil().to_num::<u64>());
    msg!(
        "bought back {} native fees with {} native mngo",
        max_buyback,
        max_buyback_mngo
    );

    emit!(AccountBuybackFeesWithMngoLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        buyback_fees: max_buyback.to_bits(),
        buyback_mngo: max_buyback_mngo.to_bits(),
        mngo_buyback_price: mngo_buyback_price.to_bits(),
        oracle_price: mngo_oracle_price.to_bits(),
    });

    // ensure dao mango account has no liabilities after we do the token swap
    for ele in dao_account.all_token_positions() {
        require!(!ele.indexed_position.is_negative(), MangoError::SomeError);
    }
    require_eq!(
        dao_account.active_perp_positions().count(),
        0,
        MangoError::SomeError
    );
    require_eq!(
        dao_account.active_serum3_orders().count(),
        0,
        MangoError::SomeError
    );

    Ok(())
}
