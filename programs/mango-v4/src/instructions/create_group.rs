use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct CreateGroup<'info> {
    #[account(
        init,
        seeds = [b"group".as_ref(), owner.key().as_ref()],
        bump,
        payer = payer,
    )]
    pub group: AccountLoader<'info, MangoGroup>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_group(ctx: Context<CreateGroup>) -> Result<()> {
    let mut group = ctx.accounts.group.load_init()?;
    group.owner = ctx.accounts.owner.key();
    Ok(())
}
