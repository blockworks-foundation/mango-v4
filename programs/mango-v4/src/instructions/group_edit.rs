use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct GroupEdit<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = group.load()?.is_operational()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,
}

// use case - transfer group ownership to governance, where
// admin and fast_listing_admin are PDAs
pub fn group_edit(
    ctx: Context<GroupEdit>,
    admin_opt: Option<Pubkey>,
    fast_listing_admin_opt: Option<Pubkey>,
    security_admin_opt: Option<Pubkey>,
    testing_opt: Option<u8>,
    version_opt: Option<u8>,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;

    if let Some(admin) = admin_opt {
        group.admin = admin;
    }

    if let Some(fast_listing_admin) = fast_listing_admin_opt {
        group.fast_listing_admin = fast_listing_admin;
    }

    if let Some(security_admin) = security_admin_opt {
        group.security_admin = security_admin;
    }

    if let Some(testing) = testing_opt {
        group.testing = testing;
    }

    if let Some(version) = version_opt {
        group.version = version;
    }

    Ok(())
}
