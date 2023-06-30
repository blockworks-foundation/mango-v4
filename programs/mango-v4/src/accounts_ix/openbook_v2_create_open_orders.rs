use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use openbook_v2::{
    program::OpenbookV2,
    state::{Market, OpenOrdersAccount},
};

#[derive(Accounts)]
#[instruction(account_num: u32, open_orders_count: u8)]
pub struct OpenbookV2CreateOpenOrders<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2CreateOpenOrders) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    pub openbook_v2_program: Program<'info, OpenbookV2>,
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    // initialized by this instruction via cpi to openbook_v2
    #[account(
        init,
        seeds = [b"OpenOrders".as_ref(), owner.key().as_ref(), account.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        owner = openbook_v2_market.key(),
        space = OpenOrdersAccount::space().unwrap(),
    )]
    /// CHECK: Newly created by openbook_v2 cpi call
    pub open_orders: AccountLoader<'info, OpenOrdersAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
