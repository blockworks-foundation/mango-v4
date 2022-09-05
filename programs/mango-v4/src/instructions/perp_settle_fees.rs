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
use crate::state::QUOTE_TOKEN_INDEX;
use crate::state::{oracle_price, AccountLoaderDynamic, Group, PerpMarket};

#[derive(Accounts)]
pub struct PerpSettleFees<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST have a loss
    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

    pub oracle: UncheckedAccount<'info>,

    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
}

pub fn perp_settle_fees(ctx: Context<PerpSettleFees>, max_settle_amount: I80F48) -> Result<()> {
    // max_settle_amount must greater than zero
    require!(
        max_settle_amount > 0,
        MangoError::MaxSettleAmountMustBeGreaterThanZero
    );

    let mut account = ctx.accounts.account.load_mut()?;
    let mut bank = ctx.accounts.quote_bank.load_mut()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    // Verify that the bank is the quote currency bank
    require!(
        bank.token_index == QUOTE_TOKEN_INDEX,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let oracle_price = oracle_price(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        perp_market.oracle_config.conf_filter,
        perp_market.base_token_decimals,
    )?;

    // Fetch perp positions for accounts
    let perp_position = account.perp_position_mut(perp_market.perp_market_index)?;

    // Settle funding before settling any PnL
    perp_position.settle_funding(&perp_market);

    // Calculate PnL for each account
    let base_native = perp_position.base_position_native(&perp_market);
    let pnl: I80F48 = cm!(perp_position.quote_position_native + base_native * oracle_price);

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
        .min(max_settle_amount);
    perp_position.quote_position_native = cm!(perp_position.quote_position_native + settlement);
    perp_market.fees_accrued = cm!(perp_market.fees_accrued - settlement);

    // Update the account's net_settled with the new PnL
    let settlement_i64 = settlement.round().checked_to_num::<i64>().unwrap();
    account.fixed.net_settled = cm!(account.fixed.net_settled - settlement_i64);

    // Transfer token balances
    // TODO: Need to guarantee that QUOTE_TOKEN_INDEX token exists at this point. I.E. create it when placing perp order.
    let token_position = account.ensure_token_position(QUOTE_TOKEN_INDEX)?.0;
    bank.withdraw_with_fee(token_position, settlement)?;
    // Update the settled balance on the market itself
    perp_market.fees_settled = cm!(perp_market.fees_settled + settlement);

    // Bank & perp_market are dropped to prevent re-borrow from remaining_accounts
    drop(bank);
    drop(perp_market);

    // Verify that the result of settling did not violate the health of the account that lost money
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let health = compute_health(&account.borrow(), HealthType::Init, &retriever)?;
    require!(health >= 0, MangoError::HealthMustBePositive);

    msg!("settled fees = {}", settlement);
    Ok(())
}
