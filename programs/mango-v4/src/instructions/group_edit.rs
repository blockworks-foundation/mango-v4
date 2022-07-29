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

// use case - transfer group ownership to governance, where
// new_admin and new_fast_listing_admin are PDAs
pub fn group_edit(
    ctx: Context<GroupEdit>,
    new_admin: Pubkey,
    new_fast_listing_admin: Pubkey,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    group.admin = new_admin;
    group.fast_listing_admin = new_fast_listing_admin;
    Ok(())
}
