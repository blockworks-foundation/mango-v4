use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::error::*;
use crate::state::*;

// Remaining accounts:
// - all banks for liab_mint_info (writable)
// - merged health accounts for liqor+liqee
#[derive(Accounts)]
pub struct TokenLiqBankruptcy<'info> {
    #[account(
        has_one = insurance_vault,
        constraint = group.load()?.is_ix_enabled(IxGate::TokenLiqBankruptcy) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.is_operational() @ MangoError::AccountIsFrozen
        // liqor_owner is checked at #1
    )]
    pub liqor: AccountLoader<'info, MangoAccountFixed>,
    pub liqor_owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = liqee.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub liqee: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        has_one = group,
    )]
    pub liab_mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    // address is checked at #2 a) and b)
    // better name would be "insurance_bank_vault"
    pub quote_vault: Account<'info, TokenAccount>,

    // future: this would be an insurance fund vault specific to a
    // trustless token, separate from the shared one on the group
    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> TokenLiqBankruptcy<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.insurance_vault.to_account_info(),
            to: self.quote_vault.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
