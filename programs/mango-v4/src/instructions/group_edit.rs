use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct GroupEdit<'info> {
    #[account(mut)]
    pub group: AccountLoader<'info, Group>,

    pub new_admin: UncheckedAccount<'info>,
    pub new_fast_listing_admin: UncheckedAccount<'info>,

    pub admin: Signer<'info>,
}

// use case - transfer group ownership to governance, where
// new_admin and new_fast_listing_admin are PDAs
pub fn group_edit(ctx: Context<GroupEdit>) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    group.admin = ctx.accounts.new_admin.key();
    group.fast_listing_admin = ctx.accounts.new_fast_listing_admin.key();
    Ok(())
}
