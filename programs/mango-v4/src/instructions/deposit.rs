use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        constraint = bank.load()?.mint == token_account.mint,
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
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

// TODO: It may make sense to have the token_index passed in from the outside.
//       That would save a lot of computation that needs to go into finding the
//       right index for the mint.
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // Find the mint's token index
    let group = ctx.accounts.group.load()?;
    let mint = ctx.accounts.token_account.mint;
    let token_index = group.tokens.index_for_mint(&mint)?;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;
    let (position, position_index) = account.indexed_positions.get_mut_or_create(token_index)?;

    // Update the bank and position
    let position_is_active = {
        let mut bank = ctx.accounts.bank.load_mut()?;
        bank.deposit(position, amount)
    };

    // Transfer the actual tokens
    token::transfer(ctx.accounts.transfer_ctx(), amount)?;

    //
    // Health computation
    // TODO: This will be used to disable is_bankrupt or being_liquidated
    //       when health recovers sufficiently
    //
    let active_len = account.indexed_positions.iter_active().count();
    require!(
        ctx.remaining_accounts.len() == active_len * 2, // banks + oracles
        MangoError::SomeError
    );

    let banks = &ctx.remaining_accounts[0..active_len];
    let oracles = &ctx.remaining_accounts[active_len..active_len * 2];

    let health = compute_health(&mut account, &banks, &oracles)?;
    msg!("health: {}", health);

    //
    // Deactivate the position only after the health check because the user passed in
    // remaining_accounts for all banks/oracles, including the account that will now be
    // deactivated.
    // Deposits can deactivate a position if they cancel out a previous borrow.
    //
    if !position_is_active {
        account.indexed_positions.deactivate(position_index);
    }

    Ok(())
}
