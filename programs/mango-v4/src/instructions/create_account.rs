use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill32_from_str;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct CreateAccount<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"MangoAccount".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MangoAccount>(),
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_account(ctx: Context<CreateAccount>, account_num: u8, name: String) -> Result<()> {
    let mut account = ctx.accounts.account.load_init()?;
    // TODO: dont init on stack
    *account = MangoAccount {
        name: fill32_from_str(name)?,
        group: ctx.accounts.group.key(),
        owner: ctx.accounts.owner.key(),
        delegate: Pubkey::default(),
        tokens: MangoAccountTokens::new(),
        serum3: MangoAccountSerum3::new(),
        perps: MangoAccountPerps::new(),
        being_liquidated: 0,
        is_bankrupt: 0,
        account_num,
        bump: *ctx.bumps.get("account").ok_or(MangoError::SomeError)?,
        reserved: Default::default(),
    };

    Ok(())
}
