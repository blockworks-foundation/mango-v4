use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TokenStopLossTrigger<'info> {
    #[account(
        // TODO: constraint = group.load()?.is_ix_enabled(IxGate::PerpPlaceOrder) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqee.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub liqee: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub liqor: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        // banks are matched to the TokenStopLoss entry at #2
    )]
    pub buy_bank: AccountLoader<'info, Bank>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    #[account(address = buy_bank.load()?.oracle)]
    pub buy_oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = group,
        // banks are matched to the TokenStopLoss entry at #2
    )]
    pub sell_bank: AccountLoader<'info, Bank>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    #[account(address = sell_bank.load()?.oracle)]
    pub sell_oracle: UncheckedAccount<'info>,
}
