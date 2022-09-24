use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::state::*;
use crate::util::checked_math as cm;

use crate::logs::{DepositLog, TokenBalanceLog};

#[derive(Accounts)]
pub struct TokenDeposit<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> TokenDeposit<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.token_account.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.token_authority.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64) -> Result<()> {
    require_msg!(amount > 0, "deposit amount must be positive");

    let token_index = ctx.accounts.bank.load()?.token_index;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;

    let (position, raw_token_index, _active_token_index) =
        account.ensure_token_position(token_index)?;
    let opening_indexed_position = position.indexed_position;

    let amount_i80f48 = I80F48::from(amount);
    let position_is_active = {
        let mut bank = ctx.accounts.bank.load_mut()?;
        bank.deposit(position, amount_i80f48)?
    };

    // Transfer the actual tokens
    token::transfer(ctx.accounts.transfer_ctx(), amount)?;

    let indexed_position = position.indexed_position;
    let bank = ctx.accounts.bank.load()?;
    let oracle_price = bank.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?)?;

    position.update_cumulative_interest(
        opening_indexed_position,
        bank.deposit_index,
        bank.borrow_index,
    );
    emit!(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index,
        indexed_position: indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
    let amount_usd = cm!(amount_i80f48 * oracle_price).to_num::<i64>();
    cm!(account.fixed.net_deposits += amount_usd);

    //
    // Health computation
    //
    // Since depositing can only increase health, we can skip the usual pre-health computation.
    // Also, TokenDeposit is one of the rare instructions that is allowed even during being_liquidated.
    //
    if !account.fixed.is_in_health_region() {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health = compute_health(&account.borrow(), HealthType::Init, &retriever)
            .context("post-deposit init health")?;
        msg!("health: {}", health);
        account.fixed.maybe_recover_from_being_liquidated(health);
    }

    //
    // Deactivate the position only after the health check because the user passed in
    // remaining_accounts for all banks/oracles, including the account that will now be
    // deactivated.
    // Deposits can deactivate a position if they cancel out a previous borrow.
    //
    if !position_is_active {
        account.deactivate_token_position(raw_token_index);
    }

    emit!(DepositLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        signer: ctx.accounts.token_authority.key(),
        token_index,
        quantity: amount,
        price: oracle_price.to_bits(),
    });

    Ok(())
}
