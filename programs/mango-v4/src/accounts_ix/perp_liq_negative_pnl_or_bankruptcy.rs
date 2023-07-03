use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

#[derive(Accounts)]
pub struct PerpLiqNegativePnlOrBankruptcy<'info> {
    #[account(
        has_one = insurance_vault,
        constraint = group.load()?.is_ix_enabled(IxGate::PerpLiqNegativePnlOrBankruptcy) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.is_operational() @ MangoError::AccountIsFrozen,
        // liqor_owner is checked at #1
    )]
    pub liqor: AccountLoader<'info, MangoAccountFixed>,
    pub liqor_owner: Signer<'info>,

    // This account MUST have a loss
    #[account(
        mut,
        has_one = group,
        constraint = liqee.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub liqee: AccountLoader<'info, MangoAccountFixed>,

    #[account(mut, has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = settle_bank.load()?.token_index == perp_market.load()?.settle_token_index @ MangoError::InvalidBank
    )]
    pub settle_bank: AccountLoader<'info, Bank>,

    #[account(
        mut,
        address = settle_bank.load()?.vault
    )]
    pub settle_vault: Account<'info, TokenAccount>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,

    // future: this would be an insurance fund vault specific to a
    // trustless token, separate from the shared one on the group
    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct PerpLiqNegativePnlOrBankruptcyV2<'info> {
    #[account(
        has_one = insurance_vault,
        constraint = group.load()?.is_ix_enabled(IxGate::PerpLiqNegativePnlOrBankruptcy) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.is_operational() @ MangoError::AccountIsFrozen,
        // liqor_owner is checked at #1
    )]
    pub liqor: AccountLoader<'info, MangoAccountFixed>,
    pub liqor_owner: Signer<'info>,

    // This account MUST have a loss
    #[account(
        mut,
        has_one = group,
        constraint = liqee.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub liqee: AccountLoader<'info, MangoAccountFixed>,

    #[account(mut, has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        constraint = settle_bank.load()?.token_index == perp_market.load()?.settle_token_index @ MangoError::InvalidBank
    )]
    pub settle_bank: AccountLoader<'info, Bank>,

    #[account(
        mut,
        address = settle_bank.load()?.vault
    )]
    pub settle_vault: Account<'info, TokenAccount>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,

    // future: this would be an insurance fund vault specific to a
    // trustless token, separate from the shared one on the group
    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        has_one = group,
        constraint = insurance_bank.load()?.token_index == INSURANCE_TOKEN_INDEX
    )]
    pub insurance_bank: AccountLoader<'info, Bank>,

    #[account(
        mut,
        address = insurance_bank.load()?.vault
    )]
    pub insurance_bank_vault: Account<'info, TokenAccount>,

    /// CHECK: Oracle can have different account types
    #[account(address = insurance_bank.load()?.oracle)]
    pub insurance_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> PerpLiqNegativePnlOrBankruptcyV2<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.insurance_vault.to_account_info(),
            to: self.insurance_bank_vault.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
