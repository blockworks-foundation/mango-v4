use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill32_from_str;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct AccountCreate<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"MangoAccount".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = MangoAccount2::space(16, 8, 8, 8),
    )]
    // borsh smashes the stack, and zero copy doesnt work out of the box
    // deserialize manually
    pub account: UncheckedAccount<'info>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn account_create(ctx: Context<AccountCreate>, account_num: u8, name: String) -> Result<()> {
    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new_init(&ctx.accounts.account)?;
    let mut account: MangoAccountAccMut = mal.load_mut()?;

    account.fixed.name = fill32_from_str(name)?;
    account.fixed.group = ctx.accounts.group.key();
    account.fixed.owner = ctx.accounts.owner.key();
    account.fixed.account_num = account_num;
    account.fixed.bump = *ctx.bumps.get("account").ok_or(MangoError::SomeError)?;
    account.fixed.delegate = Pubkey::default();
    account.fixed.set_being_liquidated(false);
    account.fixed.set_bankrupt(false);

    let token_count: u8 = 16;
    let serum3_count: u8 = 8;
    let perp_count: u8 = 8;
    let perp_oo_count: u8 = 8;
    account.expand_dynamic_content(token_count, serum3_count, perp_count, perp_oo_count)?;

    Ok(())
}
