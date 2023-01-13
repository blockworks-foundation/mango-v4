use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct GroupToggleHalt<'info> {
    #[account(
        mut,
        constraint = group.load()?.admin == admin.key() || group.load()?.security_admin == admin.key(),
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,
}

pub fn group_toggle_halt(ctx: Context<GroupToggleHalt>, halted: bool) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    group.halted = u8::from(halted);
    Ok(())
}
