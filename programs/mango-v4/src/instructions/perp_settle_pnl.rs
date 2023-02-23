use anchor_lang::prelude::*;

use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::{new_health_cache, HealthType, ScanningAccountRetriever};
use crate::logs::{emit_perp_balances, PerpSettlePnlLog, TokenBalanceLog};
use crate::state::*;

pub fn perp_settle_pnl(ctx: Context<PerpSettlePnl>) -> Result<()> {
    // Cannot settle with yourself
    require_keys_neq!(
        ctx.accounts.account_a.key(),
        ctx.accounts.account_b.key(),
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

    let a_liq_end_health;
    let a_maint_health;
    let b_settle_health;
    {
        let retriever =
            ScanningAccountRetriever::new(ctx.remaining_accounts, &ctx.accounts.group.key())
                .context("create account retriever")?;
        b_settle_health = new_health_cache(&account_b.borrow(), &retriever)?.perp_settle_health();
        let a_cache = new_health_cache(&account_a.borrow(), &retriever)?;
        a_liq_end_health = a_cache.health(HealthType::LiquidationEnd);
        a_maint_health = a_cache.health(HealthType::Maint);
    };

    let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;
    let perp_market = ctx.accounts.perp_market.load()?;

    // Verify that the bank is the quote currency bank (#2)
    require!(
        settle_bank.token_index == settle_token_index,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        None, // staleness checked in health
    )?;

    // Fetch perp position and pnl
    let a_perp_position = account_a.perp_position_mut(perp_market_index)?;
    let b_perp_position = account_b.perp_position_mut(perp_market_index)?;
    a_perp_position.settle_funding(&perp_market);
    b_perp_position.settle_funding(&perp_market);
    let a_pnl = a_perp_position.unsettled_pnl(&perp_market, oracle_price)?;
    let b_pnl = b_perp_position.unsettled_pnl(&perp_market, oracle_price)?;

    // PnL must have opposite signs for there to be a settlement:
    // Account A must be profitable, and B must be unprofitable.
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

    // Apply pnl settle limits
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    a_perp_position.update_settle_limit(&perp_market, now_ts);
    let a_settleable_pnl = a_perp_position.apply_pnl_settle_limit(&perp_market, a_pnl);
    b_perp_position.update_settle_limit(&perp_market, now_ts);
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

    // Check how much of account b's negative pnl may be actualized given the health.
    // In that, we only care about the health of spot assets on the account.
    // Example: With +100 USDC and -2 SOL (-80 USD) and -500 USD PNL the account may still settle
    //   100 - 1.1*80 = 12 USD perp pnl, even though the overall health is already negative.
    //   Further settlement would convert perp-losses into unbacked token-losses and isn't allowed.
    require_msg_typed!(
        b_settle_health >= 0,
        MangoError::HealthMustBePositive,
        "account b settle health is negative: {}",
        b_settle_health
    );

    // Settle for the maximum possible capped to target's settle health
    let settlement = a_settleable_pnl
        .min(-b_settleable_pnl)
        .min(b_settle_health)
        .max(I80F48::ZERO);
    require_msg_typed!(
        settlement >= 0,
        MangoError::SettlementAmountMustBePositive,
        "a settleable: {}, b settleable: {}, b settle health: {}",
        a_settleable_pnl,
        b_settleable_pnl,
        b_settle_health,
    );

    let fee = perp_market.compute_settle_fee(settlement, a_liq_end_health, a_maint_health)?;

    a_perp_position.record_settle(settlement);
    b_perp_position.record_settle(-settlement);
    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account_a.key(),
        a_perp_position,
        &perp_market,
    );
    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account_b.key(),
        b_perp_position,
        &perp_market,
    );

    // Update the accounts' perp_spot_transfer statistics.
    //
    // Applying the fee here means that it decreases the displayed perp pnl.
    // Think about it like this: a's pnl reduces by `settlement` and spot increases by `settlement - fee`.
    // That means that it managed to extract `settlement - fee` from perp interactions.
    let settlement_i64 = settlement.round_to_zero().to_num::<i64>();
    let fee_i64 = fee.round_to_zero().to_num::<i64>();
    (a_perp_position.perp_spot_transfers += settlement_i64 - fee_i64);
    (b_perp_position.perp_spot_transfers -= settlement_i64);
    (account_a.fixed.perp_spot_transfers += settlement_i64 - fee_i64);
    (account_b.fixed.perp_spot_transfers -= settlement_i64);

    // Transfer token balances
    // The fee is paid by the account with positive unsettled pnl
    let a_token_position = account_a.token_position_mut(settle_token_index)?.0;
    let b_token_position = account_b.token_position_mut(settle_token_index)?.0;
    settle_bank.deposit(a_token_position, settlement - fee, now_ts)?;
    // Don't charge loan origination fees on borrows created via settling:
    // Even small loan origination fees could accumulate if a perp position is
    // settled back and forth repeatedly.
    settle_bank.withdraw_without_fee(b_token_position, settlement, now_ts, oracle_price)?;

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account_a.key(),
        token_index: settle_token_index,
        indexed_position: a_token_position.indexed_position.to_bits(),
        deposit_index: settle_bank.deposit_index.to_bits(),
        borrow_index: settle_bank.borrow_index.to_bits(),
    });

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account_b.key(),
        token_index: settle_token_index,
        indexed_position: b_token_position.indexed_position.to_bits(),
        deposit_index: settle_bank.deposit_index.to_bits(),
        borrow_index: settle_bank.borrow_index.to_bits(),
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
    let settler_token_position_active = settle_bank.deposit(settler_token_position, fee, now_ts)?;

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.settler.key(),
        token_index: settler_token_position.token_index,
        indexed_position: settler_token_position.indexed_position.to_bits(),
        deposit_index: settle_bank.deposit_index.to_bits(),
        borrow_index: settle_bank.borrow_index.to_bits(),
    });

    if !settler_token_position_active {
        settler
            .deactivate_token_position_and_log(settler_token_raw_index, ctx.accounts.settler.key());
    }

    emit!(PerpSettlePnlLog {
        mango_group: ctx.accounts.group.key(),
        mango_account_a: ctx.accounts.account_a.key(),
        mango_account_b: ctx.accounts.account_b.key(),
        perp_market_index,
        settlement: settlement.to_bits(),
        settler: ctx.accounts.settler.key(),
        fee: fee.to_bits(),
    });

    msg!("settled pnl = {}, fee = {}", settlement, fee);
    Ok(())
}
