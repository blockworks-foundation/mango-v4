use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use solana_program::pubkey;

const MAIN_GROUP_ID: Pubkey = pubkey!("78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX");
const DEV1: Pubkey = pubkey!("AUTjdCY1VXMbqL2axFrbuAu4NKiYJ28zR4hWANkbkHpy");
const DEV2: Pubkey = pubkey!("DrnFiKkbyC5ga7LJDfDF8FzVcj6aoSUhsgirLjDMrBHH");

#[derive(Accounts)]
pub struct PerpSettleUnmatched<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpSettleUnmatched) @ MangoError::IxIsDisabled,
        constraint = group.key() == MAIN_GROUP_ID
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(has_one = group, has_one = oracle)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // This account MUST have a loss
    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    // bank correctness is checked at #2
    #[account(mut, has_one = group)]
    pub settle_bank: AccountLoader<'info, Bank>,

    /// CHECK: Oracle can have different account types
    #[account(address = settle_bank.load()?.oracle)]
    pub settle_oracle: UncheckedAccount<'info>,

    // This ix is intended to only be used temporarily for shutdown so we are gating access to two wallets
    #[account(
        constraint = dev.key() == DEV1 || dev.key() == DEV2
    )]
    pub dev: Signer<'info>,
}
