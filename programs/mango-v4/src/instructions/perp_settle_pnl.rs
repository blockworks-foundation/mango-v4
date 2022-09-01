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
use crate::state::TokenPosition;
use crate::state::QUOTE_TOKEN_INDEX;
use crate::state::{oracle_price, AccountLoaderDynamic, Group, PerpMarket};

#[derive(Accounts)]
pub struct PerpSettlePnl<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST be profitable
    #[account(mut, has_one = group)]
    pub account_a: AccountLoaderDynamic<'info, MangoAccount>,
    // This account MUST have a loss
    #[account(mut, has_one = group)]
    pub account_b: AccountLoaderDynamic<'info, MangoAccount>,

    pub oracle: UncheckedAccount<'info>,

    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
}

pub fn perp_settle_pnl(ctx: Context<PerpSettlePnl>, max_settle_amount: I80F48) -> Result<()> {
    // Cannot settle with yourself
    require!(
        ctx.accounts.account_a.to_account_info().key
            != ctx.accounts.account_b.to_account_info().key,
        MangoError::CannotSettleWithSelf
    );

    // max_settle_amount must greater than zero
    require!(
        max_settle_amount > 0,
        MangoError::MaxSettleAmountMustBeGreaterThanZero
    );

    let mut account_a = ctx.accounts.account_a.load_mut()?;
    let mut account_b = ctx.accounts.account_b.load_mut()?;
    let mut bank = ctx.accounts.quote_bank.load_mut()?;
    let perp_market = ctx.accounts.perp_market.load()?;

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
    let mut a_perp_position = account_a.perp_position_mut(perp_market.perp_market_index)?;
    let mut b_perp_position = account_b.perp_position_mut(perp_market.perp_market_index)?;

    // Settle funding before settling any PnL
    a_perp_position.settle_funding(&perp_market);
    b_perp_position.settle_funding(&perp_market);

    // Calculate PnL for each account
    let a_base_native = a_perp_position.base_position_native(&perp_market);
    let b_base_native = b_perp_position.base_position_native(&perp_market);
    let a_pnl: I80F48 = cm!(a_perp_position.quote_position_native + a_base_native * oracle_price);
    let b_pnl: I80F48 = cm!(b_perp_position.quote_position_native + b_base_native * oracle_price);

    // Account A must be profitable, and B must be unprofitable
    // PnL must be opposite signs for there to be a settlement
    require!(a_pnl.is_positive(), MangoError::ProfitabilityMismatch);
    require!(b_pnl.is_negative(), MangoError::ProfitabilityMismatch);

    // Settle for the maximum possible capped to max_settle_amount
    let settlement = a_pnl.abs().min(b_pnl.abs()).min(max_settle_amount);
    a_perp_position.quote_position_native = cm!(a_perp_position.quote_position_native - settlement);
    b_perp_position.quote_position_native = cm!(b_perp_position.quote_position_native + settlement);

    // Update the account's net_settled with the new PnL
    let settlement_i64 = settlement.checked_to_num::<i64>().unwrap();
    account_a.fixed.net_settled = cm!(account_a.fixed.net_settled + settlement_i64);
    account_b.fixed.net_settled = cm!(account_b.fixed.net_settled - settlement_i64);

    // Transfer token balances
    // TODO: Need to guarantee that QUOTE_TOKEN_INDEX token exists at this point. I.E. create it when placing perp order.
    let a_token_position = account_a.ensure_token_position(QUOTE_TOKEN_INDEX)?.0;
    let b_token_position = account_b.ensure_token_position(QUOTE_TOKEN_INDEX)?.0;
    transfer_token_internal(&mut bank, b_token_position, a_token_position, settlement)?;

    // Bank is dropped to prevent re-borrow from remaining_accounts
    drop(bank);

    // Verify that the result of settling did not violate the health of the account that lost money
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account_b.borrow())?;
    let health = compute_health(&account_b.borrow(), HealthType::Init, &retriever)?;
    require!(health >= 0, MangoError::HealthMustBePositive);

    msg!("settled pnl = {}", settlement);
    Ok(())
}

fn transfer_token_internal(
    bank: &mut Bank,
    from_position: &mut TokenPosition,
    to_position: &mut TokenPosition,
    native_amount: I80F48,
) -> Result<()> {
    bank.deposit(to_position, native_amount)?;
    bank.withdraw_with_fee(from_position, native_amount)?;
    Ok(())
}
