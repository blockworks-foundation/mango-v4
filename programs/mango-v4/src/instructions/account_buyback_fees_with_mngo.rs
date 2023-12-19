use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::*;

use crate::accounts_ix::*;

use crate::logs::{emit_stack, AccountBuybackFeesWithMngoLog, TokenBalanceLog};

pub fn account_buyback_fees_with_mngo(
    ctx: Context<AccountBuybackFeesWithMngo>,
    max_buyback_usd: u64,
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

    let clock = Clock::get()?;
    let now_ts = clock.unix_timestamp.try_into().unwrap();
    let slot = clock.slot;

    let mngo_oracle_ref = &AccountInfoRef::borrow(&ctx.accounts.mngo_oracle.as_ref())?;
    let mngo_oracle_price = mngo_bank.oracle_price(
        &OracleAccountInfos::from_reader(mngo_oracle_ref),
        Some(slot),
    )?;
    let mngo_asset_price = mngo_oracle_price.min(mngo_bank.stable_price());

    let fees_oracle_ref = &AccountInfoRef::borrow(&ctx.accounts.fees_oracle.as_ref())?;
    let fees_oracle_price = fees_bank.oracle_price(
        &OracleAccountInfos::from_reader(fees_oracle_ref),
        Some(slot),
    )?;
    let fees_liab_price = fees_oracle_price.max(fees_bank.stable_price());

    let bonus_factor = I80F48::from_num(group.buyback_fees_mngo_bonus_factor);

    account
        .fixed
        .expire_buyback_fees(now_ts, group.buyback_fees_expiry_interval);

    // quick return if nothing to buyback
    let mut max_buyback_usd = {
        let dao_fees_token_position = dao_account.ensure_token_position(fees_bank.token_index)?.0;
        let dao_fees = dao_fees_token_position.native(&fees_bank);
        I80F48::from_num(max_buyback_usd.min(account.fixed.buyback_fees_accrued()))
            .min(dao_fees * fees_liab_price)
    };
    if max_buyback_usd <= I80F48::ZERO {
        msg!(
            "nothing to buyback, (buyback_fees_accrued {})",
            account.fixed.buyback_fees_accrued()
        );
        return Ok(());
    }

    // if mngo token position has borrows, skip buyback
    let account_mngo = account
        .token_position(mngo_bank.token_index)
        .map(|tp| tp.native(&mngo_bank))
        .unwrap_or(I80F48::ZERO);
    if account_mngo <= I80F48::ZERO {
        msg!(
            "account mngo token position ({} native mngo) is <= 0, nothing will be bought back",
            account_mngo
        );
        return Ok(());
    }
    let (account_mngo_token_position, account_mngo_raw_token_index, _) =
        account.ensure_token_position(mngo_bank.token_index)?;

    let mngo_buyback_price = mngo_asset_price * bonus_factor;

    // compute max mngo to swap for fees
    // mngo is exchanged at a discount
    let mut max_buyback_mngo = max_buyback_usd / mngo_buyback_price;
    // buyback is restricted to account's token position
    max_buyback_mngo = max_buyback_mngo.min(account_mngo);
    max_buyback_usd = max_buyback_usd.min(max_buyback_mngo * mngo_buyback_price);
    let max_buyback_fees = max_buyback_usd / fees_liab_price;

    // move mngo from user to dao
    let (dao_mngo_token_position, dao_mngo_raw_token_index, _) =
        dao_account.ensure_token_position(mngo_bank.token_index)?;
    require!(
        dao_mngo_token_position.indexed_position >= I80F48::ZERO,
        MangoError::SomeError
    );
    let in_use =
        mngo_bank.withdraw_without_fee(account_mngo_token_position, max_buyback_mngo, now_ts)?;
    emit_stack(TokenBalanceLog {
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
    let dao_fees = dao_fees_token_position.native(&fees_bank);
    assert!(dao_fees >= max_buyback_fees);
    let in_use =
        fees_bank.withdraw_without_fee(dao_fees_token_position, max_buyback_fees, now_ts)?;
    if !in_use {
        dao_account.deactivate_token_position_and_log(
            dao_fees_raw_token_index,
            ctx.accounts.dao_account.key(),
        );
    }
    let in_use = fees_bank.deposit(account_fees_token_position, max_buyback_fees, now_ts)?;
    emit_stack(TokenBalanceLog {
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
        .reduce_buyback_fees_accrued(max_buyback_usd.ceil().to_num::<u64>());
    msg!(
        "bought back {} native usd fees by exchanging {} native mngo for {} native fees",
        max_buyback_usd,
        max_buyback_mngo,
        max_buyback_fees,
    );

    emit_stack(AccountBuybackFeesWithMngoLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        buyback_fees: max_buyback_fees.to_bits(),
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
