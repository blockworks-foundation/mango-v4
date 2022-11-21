use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::compute_health;
use crate::state::new_fixed_order_account_retriever;
use crate::state::Bank;
use crate::state::HealthType;
use crate::state::MangoAccount;
use crate::state::{AccountLoaderDynamic, Group, PerpMarket};

use crate::logs::{emit_perp_balances, PerpSettleFeesLog, TokenBalanceLog};

#[derive(Accounts)]
pub struct PerpSettleFees<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST have a loss
    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,
}

// REVIEW: max_settle_amount not needed, health check not needed
//         just compute account's perp_settle_health, and that's the max settlement
// REVIEW: This consumes negative pnl, meaning that the sum over all PNL will be more
//         positive after this instruction. Couldn't it happen that some users have positive pnl
//         but no one with negative pnl to settle against?
pub fn perp_settle_fees(ctx: Context<PerpSettleFees>, max_settle_amount: u64) -> Result<()> {
    // max_settle_amount must greater than zero
    require!(
        max_settle_amount > 0,
        MangoError::MaxSettleAmountMustBeGreaterThanZero
    );

    let mut account = ctx.accounts.account.load_mut()?;
    // REVIEW: rename settle_bank
    let mut bank = ctx.accounts.settle_bank.load_mut()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    // Verify that the bank is the quote currency bank
    require_eq!(
        bank.token_index,
        perp_market.settle_token_index,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let oracle_price = perp_market.oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        None, // staleness checked in health
    )?;

    // Fetch perp positions for accounts
    let perp_position = account.perp_position_mut(perp_market.perp_market_index)?;

    // Settle funding before settling any PnL
    perp_position.settle_funding(&perp_market);

    // Calculate PnL for each account
    let base_native = perp_position.base_position_native(&perp_market);
    let pnl: I80F48 = cm!(perp_position.quote_position_native() + base_native * oracle_price);

    // Account perp position must have a loss to be able to settle against the fee account
    require!(pnl.is_negative(), MangoError::ProfitabilityMismatch);
    require!(
        perp_market.fees_accrued.is_positive(),
        MangoError::ProfitabilityMismatch
    );

    // Settle for the maximum possible capped to max_settle_amount
    let settlement = pnl
        .abs()
        .min(perp_market.fees_accrued.abs())
        .min(I80F48::from(max_settle_amount));
    // REVIEW: require! on settlement >= 0
    perp_position.change_quote_position(settlement);
    perp_market.fees_accrued = cm!(perp_market.fees_accrued - settlement);

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account.key(),
        perp_market.perp_market_index,
        perp_position,
        &perp_market,
    );

    // Update the account's perp_spot_transfers with the new PnL
    // REVIEW: drop round()?
    let settlement_i64 = settlement.round().checked_to_num::<i64>().unwrap();
    cm!(perp_position.perp_spot_transfers -= settlement_i64);
    cm!(account.fixed.perp_spot_transfers -= settlement_i64);

    // Transfer token balances
    // REVIEW: settle_token_index == QUOTE_TOKEN_INDEX
    let token_position = account
        .token_position_mut(perp_market.settle_token_index)?
        .0;
    // REVIEW: Paying a fee here means that the account's health could go down! Is that a problem?
    bank.withdraw_with_fee(token_position, settlement)?;
    // Update the settled balance on the market itself
    perp_market.fees_settled = cm!(perp_market.fees_settled + settlement);

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index: perp_market.settle_token_index,
        indexed_position: token_position.indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    emit!(PerpSettleFeesLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        perp_market_index: perp_market.perp_market_index,
        settlement: settlement.to_bits(),
    });

    // Bank & perp_market are dropped to prevent re-borrow from remaining_accounts
    drop(bank);
    drop(perp_market);

    // Verify that the result of settling did not violate the health of the account that lost money
    // REVIEW: delete this, covered by using perp
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let health = compute_health(&account.borrow(), HealthType::Init, &retriever)?;
    require!(health >= 0, MangoError::HealthMustBePositive);

    msg!("settled fees = {}", settlement);
    Ok(())
}
