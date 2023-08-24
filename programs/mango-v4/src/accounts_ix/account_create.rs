use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(account_num: u32, token_count: u8, serum3_count: u8, perp_count: u8, perp_oo_count: u8)]
pub struct AccountCreate<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountCreate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [b"MangoAccount".as_ref(), group.key().as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = MangoAccount::space(token_count, serum3_count, perp_count, perp_oo_count, 0),
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(account_num: u32, token_count: u8, serum3_count: u8, perp_count: u8, perp_oo_count: u8, token_conditional_swap_count: u8)]
pub struct AccountCreateV2<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::AccountCreate) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [b"MangoAccount".as_ref(), group.key().as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = MangoAccount::space(token_count, serum3_count, perp_count, perp_oo_count, token_conditional_swap_count),
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
