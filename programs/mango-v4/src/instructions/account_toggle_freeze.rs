use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
pub struct AccountToggleFreeze<'info> {
    #[account(
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted,
        constraint = group.load()?.admin == admin.key() || group.load()?.security_admin == admin.key(),
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub admin: Signer<'info>,
}

pub fn account_toggle_freeze(ctx: Context<AccountToggleFreeze>, freeze: bool) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    if freeze {
        let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        account.fixed.frozen_until = now_ts + 7 * 24 * 60 * 60;
    } else {
        account.fixed.frozen_until = 0;
    }

    Ok(())
}
