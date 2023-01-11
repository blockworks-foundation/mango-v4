use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{new_health_cache, HealthType, ScanningAccountRetriever};
use crate::logs::{emit_perp_balances, PerpSettlePnlLog, TokenBalanceLog};
use crate::state::Bank;
use crate::state::{Group, MangoAccountFixed, MangoAccountLoader, PerpMarket};

#[derive(Accounts)]
pub struct PerpSettlePnl<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        // settler_owner is checked at #1
    )]
    pub settler: AccountLoader<'info, MangoAccountFixed>,
    pub settler_owner: Signer<'info>,

    #[account(has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST be profitable
    #[account(mut, has_one = group)]
    pub account_a: AccountLoader<'info, MangoAccountFixed>,
    // This account MUST have a loss
    #[account(mut, has_one = group)]
    pub account_b: AccountLoader<'info, MangoAccountFixed>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}

pub fn perp_settle_pnl(ctx: Context<PerpSettlePnl>) -> Result<()> {
    // Cannot settle with yourself
    require!(
        ctx.accounts.account_a.key() != ctx.accounts.account_b.key(),
        MangoError::CannotSettleWithSelf
    );

    let (perp_market_index, settle_token_index) = {
        let perp_market = ctx.accounts.perp_market.load()?;
        (
            perp_market.perp_market_index,
            perp_market.settle_token_index,
        )
    };

    let mut account_a = ctx.accounts.account_a.load_full_mut()?;
    let mut account_b = ctx.accounts.account_b.load_full_mut()?;

    // check positions exist, for nicer error messages
    {
        account_a.perp_position(perp_market_index)?;
        account_a.token_position(settle_token_index)?;
        account_b.perp_position(perp_market_index)?;
        account_b.token_position(settle_token_index)?;
    }

    let a_init_health;
    let a_maint_health;
    let b_settle_health;
    {
        let retriever =
            ScanningAccountRetriever::new(ctx.remaining_accounts, &ctx.accounts.group.key())
                .context("create account retriever")?;
        b_settle_health = new_health_cache(&account_b.borrow(), &retriever)?.perp_settle_health();
        let a_cache = new_health_cache(&account_a.borrow(), &retriever)?;
        a_init_health = a_cache.health(HealthType::Init);
        a_maint_health = a_cache.health(HealthType::Maint);
    };

    // Account B is the one that must have negative pnl. Check how much of that may be actualized
    // given the account's health. In that, we only care about the health of spot assets on the account.
    // Example: With +100 USDC and -2 SOL (-80 USD) and -500 USD PNL the account may still settle
    //   100 - 1.1*80 = 12 USD perp pnl, even though the overall health is already negative.
    //   Further settlement would convert perp-losses into token-losses and isn't allowed.
    require!(b_settle_health >= 0, MangoError::HealthMustBePositive);

    let mut bank = ctx.accounts.settle_bank.load_mut()?;
    let perp_market = ctx.accounts.perp_market.load()?;

    // Verify that the bank is the quote currency bank
    require!(
        bank.token_index == settle_token_index,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        None, // staleness checked in health
    )?;

    // Fetch perp positions for accounts
    let a_perp_position = account_a.perp_position_mut(perp_market_index)?;
    let b_perp_position = account_b.perp_position_mut(perp_market_index)?;

    // Settle funding before settling any PnL
    a_perp_position.settle_funding(&perp_market);
    b_perp_position.settle_funding(&perp_market);

    // Calculate PnL for each account
    let a_pnl = a_perp_position.pnl_for_price(&perp_market, oracle_price)?;
    let b_pnl = b_perp_position.pnl_for_price(&perp_market, oracle_price)?;

    // Account A must be profitable, and B must be unprofitable
    // PnL must be opposite signs for there to be a settlement
    require_msg_typed!(
        a_pnl.is_positive(),
        MangoError::ProfitabilityMismatch,
        "account a pnl is not positive: {}",
        a_pnl
    );
    require_msg_typed!(
        b_pnl.is_negative(),
        MangoError::ProfitabilityMismatch,
        "account b pnl is not negative: {}",
        b_pnl
    );

    // Cap settlement of unrealized pnl
    // Settles at most x100% each hour
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    a_perp_position.update_settle_limit(&perp_market, now_ts);
    b_perp_position.update_settle_limit(&perp_market, now_ts);
    let a_settleable_pnl = a_perp_position.apply_pnl_settle_limit(&perp_market, a_pnl);
    let b_settleable_pnl = b_perp_position.apply_pnl_settle_limit(&perp_market, b_pnl);

    require_msg_typed!(
        a_settleable_pnl.is_positive(),
        MangoError::ProfitabilityMismatch,
        "account a settleable pnl is not positive: {}, pnl: {}",
        a_settleable_pnl,
        a_pnl
    );
    require_msg_typed!(
        b_settleable_pnl.is_negative(),
        MangoError::ProfitabilityMismatch,
        "account b settleable pnl is not negative: {}, pnl: {}",
        b_settleable_pnl,
        b_pnl
    );

    // Settle for the maximum possible capped to b's settle health
    let settlement = a_settleable_pnl
        .abs()
        .min(b_settleable_pnl.abs())
        .min(b_settle_health);
    require_msg_typed!(
        settlement >= 0,
        MangoError::SettlementAmountMustBePositive,
        "a settleable: {}, b settleable: {}, b settle health: {}",
        a_settleable_pnl,
        b_settleable_pnl,
        b_settle_health,
    );

    // Settle
    a_perp_position.record_settle(settlement);
    b_perp_position.record_settle(-settlement);

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account_a.key(),
        perp_market.perp_market_index,
        a_perp_position,
        &perp_market,
    );

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account_b.key(),
        perp_market.perp_market_index,
        b_perp_position,
        &perp_market,
    );

    // A percentage fee is paid to the settler when account_a's health is low.
    // That's because the settlement could avoid it getting liquidated.
    let low_health_fee = if a_init_health < 0 {
        let fee_fraction = I80F48::from_num(perp_market.settle_fee_fraction_low_health);
        if a_maint_health < 0 {
            cm!(settlement * fee_fraction)
        } else {
            cm!(settlement * fee_fraction * (-a_init_health / (a_maint_health - a_init_health)))
        }
    } else {
        I80F48::ZERO
    };

    // The settler receives a flat fee
    let flat_fee = I80F48::from_num(perp_market.settle_fee_flat);

    // Fees only apply when the settlement is large enough
    let fee = if settlement >= perp_market.settle_fee_amount_threshold {
        cm!(low_health_fee + flat_fee).min(settlement)
    } else {
        I80F48::ZERO
    };

    // Safety check to prevent any accidental negative transfer
    require!(fee >= 0, MangoError::SettlementAmountMustBePositive);

    // Update the account's net_settled with the new PnL.
    // Applying the fee here means that it decreases the displayed perp pnl.
    let settlement_i64 = settlement.round_to_zero().checked_to_num::<i64>().unwrap();
    let fee_i64 = fee.round_to_zero().checked_to_num::<i64>().unwrap();
    cm!(a_perp_position.perp_spot_transfers += settlement_i64 - fee_i64);
    cm!(b_perp_position.perp_spot_transfers -= settlement_i64);
    cm!(account_a.fixed.perp_spot_transfers += settlement_i64 - fee_i64);
    cm!(account_b.fixed.perp_spot_transfers -= settlement_i64);

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    // Transfer token balances
    // The fee is paid by the account with positive unsettled pnl
    let a_token_position = account_a.token_position_mut(settle_token_index)?.0;
    let b_token_position = account_b.token_position_mut(settle_token_index)?.0;
    bank.deposit(a_token_position, cm!(settlement - fee), now_ts)?;
    // Don't charge loan origination fees on borrows created via settling:
    // Even small loan origination fees could accumulate if a perp position is
    // settled back and forth repeatedly.
    bank.withdraw_without_fee(b_token_position, settlement, now_ts, oracle_price)?;

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.settler.key(),
        token_index: settle_token_index,
        indexed_position: a_token_position.indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.settler.key(),
        token_index: settle_token_index,
        indexed_position: b_token_position.indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    // settler might be the same as account a or b
    drop(account_a);
    drop(account_b);

    let mut settler = ctx.accounts.settler.load_full_mut()?;
    // account constraint #1
    require!(
        settler
            .fixed
            .is_owner_or_delegate(ctx.accounts.settler_owner.key()),
        MangoError::SomeError
    );

    let (settler_token_position, settler_token_raw_index, _) =
        settler.ensure_token_position(settle_token_index)?;
    let settler_token_position_active = bank.deposit(settler_token_position, fee, now_ts)?;

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.settler.key(),
        token_index: settler_token_position.token_index,
        indexed_position: settler_token_position.indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    if !settler_token_position_active {
        settler
            .deactivate_token_position_and_log(settler_token_raw_index, ctx.accounts.settler.key());
    }

    emit!(PerpSettlePnlLog {
        mango_group: ctx.accounts.group.key(),
        mango_account_a: ctx.accounts.account_a.key(),
        mango_account_b: ctx.accounts.account_b.key(),
        perp_market_index: perp_market_index,
        settlement: settlement.to_bits(),
        settler: ctx.accounts.settler.key(),
        fee: fee.to_bits(),
    });

    msg!("settled pnl = {}, fee = {}", settlement, fee);
    Ok(())
}
