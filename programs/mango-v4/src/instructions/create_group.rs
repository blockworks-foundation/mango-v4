use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(group_num: u32)]
pub struct CreateGroup<'info> {
    #[account(
        init,
        seeds = [b"Group".as_ref(), admin.key().as_ref(), &group_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Group>(),
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_group(ctx: Context<CreateGroup>, group_num: u32) -> Result<()> {
    let mut group = ctx.accounts.group.load_init()?;
    group.admin = ctx.accounts.admin.key();
    group.bump = *ctx.bumps.get("group").ok_or(MangoError::SomeError)?;
    group.group_num = group_num;
    Ok(())
}
