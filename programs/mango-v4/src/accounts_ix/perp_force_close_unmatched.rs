use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use solana_program::pubkey;
/*
This is a special instruction created to clean up the unmatchable 2 BTC-PERP base lot positions in the main mango group.
This instruction can only be used for that group and only for those 2 BTC-PERP longs
The instruction will close out the BTC-PERP position in each
 */
const MAIN_GROUP_ID: Pubkey = pubkey!("78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX");
const BTC_PERP_MARKET_PUBKEY: Pubkey = pubkey!("HwhVGkfsSQ9JSQeQYu2CbkRCLvsh3qRZxG6m4oMVwZpN");
const DEV1: Pubkey = pubkey!("AUTjdCY1VXMbqL2axFrbuAu4NKiYJ28zR4hWANkbkHpy");
const DEV2: Pubkey = pubkey!("DrnFiKkbyC5ga7LJDfDF8FzVcj6aoSUhsgirLjDMrBHH");
#[derive(Accounts)]
pub struct PerpForceCloseUnmatched<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpForceCloseUnmatched) @ MangoError::IxIsDisabled,
        constraint = group.key() == MAIN_GROUP_ID
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = oracle,
        constraint = perp_market.load()?.is_force_close(),
        constraint = perp_market.key() == BTC_PERP_MARKET_PUBKEY
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    // Owner will not be checked, this a forced operation
    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    /// CHECK: Oracle can have different account types, constrained by address in perp_market
    pub oracle: UncheckedAccount<'info>,

    // This ix is intended to only be used temporarily for shutdown so we are gating access to two wallets
    #[account(
        constraint = dev.key() == DEV1 || dev.key() == DEV2
    )]
    pub dev: Signer<'info>,
}
