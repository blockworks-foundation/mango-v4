use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;

use crate::logs::{TokenBalanceLog, WithdrawLog};
use crate::state::new_fixed_order_account_retriever;

#[derive(Accounts)]
pub struct TokenWithdraw<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        // note: should never be the delegate
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> TokenWithdraw<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.vault.to_account_info(),
            to: self.token_account.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

// TODO: It may make sense to have the token_index passed in from the outside.
//       That would save a lot of computation that needs to go into finding the
//       right index for the mint.
// TODO: https://github.com/blockworks-foundation/mango-v4/commit/15961ec81c7e9324b37d79d0e2a1650ce6bd981d comments
pub fn token_withdraw(ctx: Context<TokenWithdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
    require!(amount > 0, MangoError::SomeError);

    let group = ctx.accounts.group.load()?;
    let token_index = ctx.accounts.bank.load()?.token_index;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;
    require!(account.is_bankrupt == 0, MangoError::IsBankrupt);

    let (position, raw_token_index, active_token_index) =
        account.tokens.get_mut_or_create(token_index)?;

    // The bank will also be passed in remainingAccounts. Use an explicit scope
    // to drop the &mut before we borrow it immutably again later.
    let position_is_active = {
        let mut bank = ctx.accounts.bank.load_mut()?;
        let native_position = position.native(&bank);

        // Handle amount special case for withdrawing everything
        let amount = if amount == u64::MAX && !allow_borrow {
            if native_position.is_positive() {
                // TODO: This rounding may mean that if we deposit and immediately withdraw
                //       we can't withdraw the full amount!
                native_position.floor().to_num::<u64>()
            } else {
                return Ok(());
            }
        } else {
            amount
        };

        require!(
            allow_borrow || amount < native_position,
            MangoError::SomeError
        );

        let amount_i80f48 = I80F48::from(amount);

        // Update the bank and position
        let position_is_active = bank.withdraw_with_fee(position, amount_i80f48)?;

        // Transfer the actual tokens
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            amount,
        )?;

        position_is_active
    };

    let indexed_position = position.indexed_position;

    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account)?;
    let (bank, oracle_price) =
        retriever.bank_and_oracle(&ctx.accounts.group.key(), active_token_index, token_index)?;
    emit!(TokenBalanceLog {
        mango_account: ctx.accounts.account.key(),
        token_index: token_index,
        indexed_position: indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
        price: oracle_price.to_bits(),
    });

    //
    // Health check
    //
    let health = compute_health(&account, HealthType::Init, &retriever)?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::HealthMustBePositive);

    //
    // Deactivate the position only after the health check because the user passed in
    // remaining_accounts for all banks/oracles, including the account that will now be
    // deactivated.
    //
    if !position_is_active {
        account.tokens.deactivate(raw_token_index);
    }

    emit!(WithdrawLog {
        mango_account: ctx.accounts.account.key(),
        signer: ctx.accounts.owner.key(),
        token_index: token_index,
        quantity: amount,
        price: oracle_price.to_bits(),
    });

    Ok(())
}
