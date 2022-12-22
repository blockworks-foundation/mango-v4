use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;

use crate::error::MangoError;
use crate::state::*;

pub mod compute_budget {
    use solana_program::declare_id;
    declare_id!("ComputeBudget111111111111111111111111111111");
}

/// Updates token interest and interest rates.
///
/// In addition to these accounts, all banks must be passed as remaining_accounts
/// in MintInfo order.
///
/// This instruction may only be used alongside other instructions of the same kind
/// or ComputeBudget instructions.
#[derive(Accounts)]
pub struct TokenUpdateIndexAndRate<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TokenUpdateIndexAndRate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>, // Required for group metadata parsing

    #[account(
        has_one = oracle,
        has_one = group,
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    /// CHECK: oracle can be one of multiple account types
    pub oracle: UncheckedAccount<'info>,

    /// CHECK: fixed instructions sysvar account
    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}
