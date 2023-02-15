use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ComputeAccountData<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(has_one = group)]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}
