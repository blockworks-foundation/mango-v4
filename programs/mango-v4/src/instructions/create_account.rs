use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct CreateAccount<'info> {
    pub group: AccountLoader<'info, Group>,

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
}

pub fn create_account(ctx: Context<CreateAccount>, account_num: u8) -> Result<()> {
    let mut account = ctx.accounts.account.load_init()?;
    *account = MangoAccount {
        group: ctx.accounts.group.key(),
        owner: ctx.accounts.owner.key(),
        delegate: Pubkey::default(),
        indexed_positions: IndexedPositions::new(),
        serum_open_orders_map: SerumOpenOrdersMap::new(),
        being_liquidated: false,
        is_bankrupt: false,
        account_num,
        bump: *ctx.bumps.get("account").ok_or(MangoError::SomeError)?,
        reserved: [0; 5],
    };

    Ok(())
}
