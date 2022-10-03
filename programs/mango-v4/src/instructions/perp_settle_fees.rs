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
    pub quote_bank: AccountLoader<'info, Bank>,
}

pub fn perp_settle_fees(ctx: Context<PerpSettleFees>, max_settle_amount: u64) -> Result<()> {
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
    let oracle_price =
        perp_market.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

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
    perp_position.change_quote_position(settlement);
    perp_market.fees_accrued = cm!(perp_market.fees_accrued - settlement);

    // Update the account's perp_spot_transfers with the new PnL
    let settlement_i64 = settlement.round().checked_to_num::<i64>().unwrap();
    account.fixed.perp_spot_transfers = cm!(account.fixed.perp_spot_transfers - settlement_i64);

    // Transfer token balances
    // TODO: Need to guarantee that QUOTE_TOKEN_INDEX token exists at this point. I.E. create it when placing perp order.
    let token_position = account.ensure_token_position(QUOTE_TOKEN_INDEX)?.0;
    bank.withdraw_with_fee(token_position, settlement)?;
    // Update the settled balance on the market itself
    perp_market.fees_settled = cm!(perp_market.fees_settled + settlement);

    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index: QUOTE_TOKEN_INDEX,
        indexed_position: token_position.indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.account.key(),
        perp_market.perp_market_index,
        account
            .perp_position(perp_market.perp_market_index)
            .unwrap(),
        &perp_market,
    );

    emit!(PerpSettleFeesLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        market_index: perp_market.perp_market_index,
        settlement: settlement.to_bits(),
    });

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
