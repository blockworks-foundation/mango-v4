use crate::{accounts_ix::*, error::MangoError};
use anchor_lang::prelude::*;

pub fn serum3_edit_market(
    ctx: Context<Serum3EditMarket>,
    reduce_only_opt: Option<bool>,
) -> Result<()> {
    let mut serum3_market = ctx.accounts.market.load_mut()?;

    let group = ctx.accounts.group.load()?;
    let mut require_group_admin = false;

    if let Some(reduce_only) = reduce_only_opt {
        msg!(
            "Reduce only: old - {:?}, new - {:?}",
            serum3_market.reduce_only,
            u8::from(reduce_only)
        );
        serum3_market.reduce_only = u8::from(reduce_only);

        // security admin can only enable reduce_only
        if !reduce_only {
            require_group_admin = true;
        }
    };

    if require_group_admin {
        require!(
            group.admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    } else {
        require!(
            group.admin == ctx.accounts.admin.key()
                || group.security_admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    }

    Ok(())
}
