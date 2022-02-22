use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"tokenbank".as_ref(), deposit_token.mint.as_ref()],
        // TODO: not sure if getting the bump like this is worth it!
        bump = group.load()?.tokens.info_for_mint(&deposit_token.mint)?.bank_bump,
    )]
    pub bank: AccountLoader<'info, TokenBank>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"tokenvault".as_ref(), deposit_token.mint.as_ref()],
        // TODO: not sure if getting the bump like this is worth it!
        bump = group.load()?.tokens.info_for_mint(&deposit_token.mint)?.vault_bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub deposit_token: Box<Account<'info, TokenAccount>>,
    pub deposit_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.deposit_token.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.deposit_authority.to_account_info(),
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
    let mint = ctx.accounts.deposit_token.mint;
    let token_index = group.tokens.index_for_mint(&mint)?;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;
    let position = account.indexed_positions.get_mut_or_create(token_index)?;

    // Update the bank and position
    let mut bank = ctx.accounts.bank.load_mut()?;
    bank.deposit(position, amount);

    // Transfer the actual tokens
    token::transfer(ctx.accounts.transfer_ctx(), amount)?;

    Ok(())
}
