use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

#[derive(Accounts)]
pub struct OpenbookV2PlaceOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2PlaceOrder) @ MangoError::IxIsDisabled,
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

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = openbook_v2_program,
        has_one = openbook_v2_market_external,
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,
    /// CHECK: The pubkey is checked and then it's passed to the openbook_v2 cpi
    pub openbook_v2_program: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: The pubkey is checked and then it's passed to the openbook_v2 cpi
    pub openbook_v2_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the openbook_v2 cpi call
    // and are validated there.
    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_event_queue: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_request_queue: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_quote_vault: UncheckedAccount<'info>,
    /// needed for the automatic settle_funds call
    /// CHECK: Validated by the openbook_v2 cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    /// The bank that pays for the order, if necessary
    // token_index and payer_bank.vault == payer_vault is validated inline at #3
    #[account(mut, has_one = group)]
    pub payer_bank: AccountLoader<'info, Bank>,
    /// The bank vault that pays for the order, if necessary
    #[account(mut)]
    pub payer_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: The oracle can be one of several different account types
    #[account(address = payer_bank.load()?.oracle)]
    pub payer_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
