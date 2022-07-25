use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill32_from_str;

#[derive(Accounts)]
#[instruction(account_num: u8, account_size: AccountSize)]
pub struct AccountCreate<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"MangoAccount".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = MangoAccount::space(account_size),
    )]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn account_create(
    ctx: Context<AccountCreate>,
    account_num: u8,
    account_size: AccountSize,
    name: String,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_init()?;

    account.fixed.name = fill32_from_str(name)?;
    account.fixed.group = ctx.accounts.group.key();
    account.fixed.owner = ctx.accounts.owner.key();
    account.fixed.account_num = account_num;
    account.fixed.bump = *ctx.bumps.get("account").ok_or(MangoError::SomeError)?;
    account.fixed.delegate = Pubkey::default();
    account.fixed.set_being_liquidated(false);
    account.fixed.set_bankrupt(false);

    account.expand_dynamic_content(account_size)?;

    Ok(())
}
