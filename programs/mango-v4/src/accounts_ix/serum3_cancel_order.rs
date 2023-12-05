use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3CancelOrder<'info> {
    // ix enabled check is done in instructions
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_event_queue: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Serum3CancelOrderV2Extra<'info> {
    #[account(mut)]
    pub quote_bank: AccountLoader<'info, Bank>,
    /// CHECK: The oracle can be one of several different account types and the pubkey is checked
    #[account(address = quote_bank.load()?.oracle)]
    pub quote_oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub base_bank: AccountLoader<'info, Bank>,
    /// CHECK: The oracle can be one of several different account types and the pubkey is checked
    #[account(address = base_bank.load()?.oracle)]
    pub base_oracle: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Serum3CancelOrderV2<'info> {
    pub v1: Serum3CancelOrder<'info>,
    #[account(
        constraint = v2.quote_bank.load()?.group == v1.group.key(),
        constraint = v2.quote_bank.load()?.token_index == v1.serum_market.load()?.quote_token_index,
        constraint = v2.base_bank.load()?.group == v1.group.key(),
        constraint = v2.base_bank.load()?.token_index == v1.serum_market.load()?.base_token_index,
    )]
    pub v2: Serum3CancelOrderV2Extra<'info>,
}
