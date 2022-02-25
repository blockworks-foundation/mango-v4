use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"tokenbank".as_ref(), token_account.mint.as_ref()],
        // TODO: not sure if getting the bump like this is worth it!
        bump = group.load()?.tokens.info_for_mint(&token_account.mint)?.bank_bump,
    )]
    pub bank: AccountLoader<'info, TokenBank>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"tokenvault".as_ref(), token_account.mint.as_ref()],
        // TODO: not sure if getting the bump like this is worth it!
        bump = group.load()?.tokens.info_for_mint(&token_account.mint)?.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
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
pub fn withdraw(ctx: Context<Withdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
    // Find the mint's token index
    let group = ctx.accounts.group.load()?;
    let mint = ctx.accounts.token_account.mint;
    let token_index = group.tokens.index_for_mint(&mint)?;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;
    let position = account.indexed_positions.get_mut_or_create(token_index)?;

    let amount = if allow_borrow {
        amount
    } else {
        // TODO: compute limit
        0
    };

    // Update the bank and position
    let mut bank = ctx.accounts.bank.load_mut()?;
    bank.withdraw(position, amount);

    // Transfer the actual tokens
    let group_seeds = group_seeds!(group);
    token::transfer(ctx.accounts.transfer_ctx().with_signer(&[group_seeds]), amount)?;

    // TODO: Health check

    Ok(())
}
