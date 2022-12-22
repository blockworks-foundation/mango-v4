use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;

pub fn group_create(
    ctx: Context<GroupCreate>,
    group_num: u32,
    testing: u8,
    version: u8,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_init()?;
    group.creator = ctx.accounts.creator.key();
    group.group_num = group_num;
    group.admin = ctx.accounts.creator.key();
    group.fast_listing_admin = Pubkey::default();
    group.insurance_vault = ctx.accounts.insurance_vault.key();
    group.insurance_mint = ctx.accounts.insurance_mint.key();
    group.bump = *ctx.bumps.get("group").ok_or(MangoError::SomeError)?;
    group.testing = testing;
    group.version = version;
    Ok(())
}
