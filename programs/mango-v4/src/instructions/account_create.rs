use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill_from_str;

#[derive(Accounts)]
#[instruction(account_num: u32, token_count: u8, serum3_count: u8, perp_count: u8, perp_oo_count: u8)]
pub struct AccountCreate<'info> {
    #[account(
        constraint = group.load()?.is_operational() @ MangoError::GroupIsHalted
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [b"MangoAccount".as_ref(), group.key().as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = MangoAccount::space(token_count, serum3_count, perp_count, perp_oo_count)?,
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

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
