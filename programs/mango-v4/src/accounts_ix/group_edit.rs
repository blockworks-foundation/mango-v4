use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct GroupEdit<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,
}
