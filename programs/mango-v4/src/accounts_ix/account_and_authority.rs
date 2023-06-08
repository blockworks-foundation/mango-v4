use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct AccountAndAuthority<'info> {
    // Instructions using this must put the ix gate into instruction code!
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen,
        constraint = account.load()?.is_owner_or_delegate(authority.key()),
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub authority: Signer<'info>,
}
