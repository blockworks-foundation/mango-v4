use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;
use crate::util::fill_from_str;

pub fn account_create(
    ctx: Context<AccountCreate>,
    account_num: u32,
    token_count: u8,
    serum3_count: u8,
    perp_count: u8,
    perp_oo_count: u8,
    name: String,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_init()?;

    msg!(
        "Initialized account with header version {}",
        account.header_version()
    );

    account.fixed.name = fill_from_str(&name)?;
    account.fixed.group = ctx.accounts.group.key();
    account.fixed.owner = ctx.accounts.owner.key();
    account.fixed.account_num = account_num;
    account.fixed.bump = *ctx.bumps.get("account").ok_or(MangoError::SomeError)?;
    account.fixed.delegate = Pubkey::default();
    account.fixed.set_being_liquidated(false);

    account.expand_dynamic_content(token_count, serum3_count, perp_count, perp_oo_count)?;

    Ok(())
}
