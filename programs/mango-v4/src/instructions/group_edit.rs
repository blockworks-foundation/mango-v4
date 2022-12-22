use anchor_lang::prelude::*;

use crate::accounts_ix::*;

// use case - transfer group ownership to governance, where
// admin and fast_listing_admin are PDAs
pub fn group_edit(
    ctx: Context<GroupEdit>,
    admin_opt: Option<Pubkey>,
    fast_listing_admin_opt: Option<Pubkey>,
    security_admin_opt: Option<Pubkey>,
    testing_opt: Option<u8>,
    version_opt: Option<u8>,
    deposit_limit_quote_opt: Option<u64>,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;

    if let Some(admin) = admin_opt {
        require_keys_neq!(admin, Pubkey::default());
        msg!("Admin old {:?}, new {:?}", group.admin, admin);
        group.admin = admin;
    }

    if let Some(fast_listing_admin) = fast_listing_admin_opt {
        msg!(
            "Fast listing admin old {:?}, new {:?}",
            group.fast_listing_admin,
            fast_listing_admin
        );
        group.fast_listing_admin = fast_listing_admin;
    }

    if let Some(security_admin) = security_admin_opt {
        msg!(
            "Security admin old {:?}, new {:?}",
            group.security_admin,
            security_admin
        );
        group.security_admin = security_admin;
    }

    if let Some(testing) = testing_opt {
        msg!("Testing old {:?}, new {:?}", group.testing, testing);
        group.testing = testing;
    }

    if let Some(version) = version_opt {
        msg!("Version old {:?}, new {:?}", group.version, version);
        group.version = version;
    }

    if let Some(deposit_limit_quote) = deposit_limit_quote_opt {
        msg!(
            "Deposit limit quote old {:?}, new {:?}",
            group.deposit_limit_quote,
            deposit_limit_quote
        );
        group.deposit_limit_quote = deposit_limit_quote;
    }

    Ok(())
}
