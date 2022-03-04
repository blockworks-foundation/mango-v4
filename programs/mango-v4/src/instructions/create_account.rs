use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct CreateAccount<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"account".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MangoAccount>(),
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_account(ctx: Context<CreateAccount>, account_num: u8) -> Result<()> {
    let mut account = ctx.accounts.account.load_init()?;
    account.group = ctx.accounts.group.key();
    account.owner = ctx.accounts.owner.key();
    account.account_num = account_num;
    account.indexed_positions = IndexedPositions::new();
    account.bump = *ctx.bumps.get("account").ok_or(MangoError::SomeError)?;

    Ok(())
}
