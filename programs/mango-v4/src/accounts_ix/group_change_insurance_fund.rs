use crate::{error::MangoError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct GroupChangeInsuranceFund<'info> {
    #[account(
        mut,
        has_one = insurance_vault,
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::GroupChangeInsuranceFund) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        close = payer,
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub withdraw_destination: Account<'info, TokenAccount>,

    pub new_insurance_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"InsuranceVault".as_ref(), group.key().as_ref(), new_insurance_mint.key().as_ref()],
        bump,
        token::authority = group,
        token::mint = new_insurance_mint,
        payer = payer
    )]
    pub new_insurance_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> GroupChangeInsuranceFund<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.insurance_vault.to_account_info(),
            to: self.withdraw_destination.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }

    pub fn close_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::CloseAccount<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            token::CloseAccount {
                account: self.insurance_vault.to_account_info(),
                destination: self.payer.to_account_info(),
                authority: self.group.to_account_info(),
            },
        )
    }
}
