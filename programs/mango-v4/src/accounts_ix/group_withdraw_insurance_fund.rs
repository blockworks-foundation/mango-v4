use crate::{error::MangoError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

#[derive(Accounts)]
pub struct GroupWithdrawInsuranceFund<'info> {
    #[account(
        has_one = insurance_vault,
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::GroupWithdrawInsuranceFund) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> GroupWithdrawInsuranceFund<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.insurance_vault.to_account_info(),
            to: self.destination.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
