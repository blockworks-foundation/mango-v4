use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct IxGateSet<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,
}

pub fn ix_gate_set(ctx: Context<IxGateSet>, ix_gate: u128) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    msg!("old  {:?}, new {:?}", group.ix_gate, ix_gate);
    group.ix_gate = ix_gate;
    Ok(())
}
