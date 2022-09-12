use anchor_lang::prelude::*;
use checked_math as cm;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::new_fixed_order_account_retriever;
use crate::state::new_health_cache;
use crate::state::Bank;
use crate::state::HealthType;
use crate::state::MangoAccount;
use crate::state::TokenPosition;
use crate::state::QUOTE_TOKEN_INDEX;
use crate::state::{AccountLoaderDynamic, Group, PerpMarket};

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

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
}

pub fn perp_settle_pnl(ctx: Context<PerpSettlePnl>, max_settle_amount: u64) -> Result<()> {
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

    let perp_market_index = {
        let perp_market = ctx.accounts.perp_market.load()?;
        perp_market.perp_market_index
    };

    let mut account_a = ctx.accounts.account_a.load_mut()?;
    let mut account_b = ctx.accounts.account_b.load_mut()?;

    // check positions exist, for nicer error messages
    {
        account_a.perp_position(perp_market_index)?;
        account_a.token_position(QUOTE_TOKEN_INDEX)?;
        account_b.perp_position(perp_market_index)?;
        account_b.token_position(QUOTE_TOKEN_INDEX)?;
    }

    // Account B is the one that must have negative pnl. Check how much of that may be actualized
    // given the account's health. In that, we only care about the health of spot assets on the account.
    // Example: With +100 USDC and -2 SOL (-80 USD) and -500 USD PNL the account may still settle
    //   100 - 1.1*80 = 12 USD perp pnl, even though the overall health is already negative.
    //   Afterwards the account is perp-bankrupt.
    let b_spot_health = {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account_b.borrow())?;
        new_health_cache(&account_b.borrow(), &retriever)?.spot_health(HealthType::Maint)
    };
    require!(b_spot_health >= 0, MangoError::HealthMustBePositive);

    let mut bank = ctx.accounts.quote_bank.load_mut()?;
    let perp_market = ctx.accounts.perp_market.load()?;

    // Verify that the bank is the quote currency bank
    require!(
        bank.token_index == QUOTE_TOKEN_INDEX,
        MangoError::InvalidBank
    );

    // Get oracle price for market. Price is validated inside
    let oracle_price =
        perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

    // Fetch perp positions for accounts
    let a_perp_position = account_a.perp_position_mut(perp_market_index)?;
    let b_perp_position = account_b.perp_position_mut(perp_market_index)?;

    // Settle funding before settling any PnL
    a_perp_position.settle_funding(&perp_market);
    b_perp_position.settle_funding(&perp_market);

    // Calculate PnL for each account
    let a_base_native = a_perp_position.base_position_native(&perp_market);
    let b_base_native = b_perp_position.base_position_native(&perp_market);
    let a_pnl: I80F48 = cm!(a_perp_position.quote_position_native() + a_base_native * oracle_price);
    let b_pnl: I80F48 = cm!(b_perp_position.quote_position_native() + b_base_native * oracle_price);

    // Account A must be profitable, and B must be unprofitable
    // PnL must be opposite signs for there to be a settlement
    require!(a_pnl.is_positive(), MangoError::ProfitabilityMismatch);
    require!(b_pnl.is_negative(), MangoError::ProfitabilityMismatch);

    // Settle for the maximum possible capped to max_settle_amount and b's spot health
    let settlement = a_pnl
        .abs()
        .min(b_pnl.abs())
        .min(b_spot_health)
        .min(I80F48::from(max_settle_amount));
    a_perp_position.change_quote_position(-settlement);
    b_perp_position.change_quote_position(settlement);

    // Update the account's net_settled with the new PnL
    let settlement_i64 = settlement.checked_to_num::<i64>().unwrap();
    cm!(account_a.fixed.net_settled += settlement_i64);
    cm!(account_b.fixed.net_settled -= settlement_i64);

    // Transfer token balances
    let a_token_position = account_a.token_position_mut(QUOTE_TOKEN_INDEX)?.0;
    let b_token_position = account_b.token_position_mut(QUOTE_TOKEN_INDEX)?.0;
    transfer_token_internal(&mut bank, b_token_position, a_token_position, settlement)?;

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
