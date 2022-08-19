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
    new_admin_opt: Option<Pubkey>,
    new_fast_listing_admin_opt: Option<Pubkey>,
    testing_opt: Option<u8>,
    version_opt: Option<u8>,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;

    if let Some(new_admin) = new_admin_opt {
        group.admin = new_admin;
    }

    if let Some(new_fast_listing_admin) = new_fast_listing_admin_opt {
        group.fast_listing_admin = new_fast_listing_admin;
    }

    if let Some(testing) = testing_opt {
        group.testing = testing;
    }

    if let Some(version) = version_opt {
        group.version = version;
    }
    Ok(())
}
