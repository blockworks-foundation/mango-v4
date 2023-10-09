use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct AdminPerpWithdrawFees<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AdminPerpWithdrawFees) @ MangoError::IxIsDisabled,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        constraint = bank.load()?.token_index == perp_market.load()?.settle_token_index
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    pub admin: Signer<'info>,
}

impl<'info> AdminPerpWithdrawFees<'info> {
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
