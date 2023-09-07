use crate::error::MangoError;
use crate::state::*;
use anchor_lang::prelude::*;
use openbook_v2::{program::OpenbookV2, state::Market};

#[derive(Accounts)]
#[instruction(market_index: OpenbookV2MarketIndex)]
pub struct OpenbookV2RegisterMarket<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = group.load()?.is_ix_enabled(IxGate::OpenbookV2RegisterMarket) @ MangoError::IxIsDisabled,
        constraint = group.load()?.openbook_v2_supported()
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    /// CHECK: Can register a market for any openbook_v2 program
    pub openbook_v2_program: Program<'info, OpenbookV2>,

    #[account(
        constraint = openbook_v2_market_external.load()?.base_mint == base_bank.load()?.mint,
        constraint = openbook_v2_market_external.load()?.quote_mint == quote_bank.load()?.mint,
    )]
    pub openbook_v2_market_external: AccountLoader<'info, Market>,

    #[account(
        init,
        // using the openbook_v2_market_external in the seed guards against registering the same market twice
        seeds = [b"OpenbookV2Market".as_ref(), group.key().as_ref(), openbook_v2_market_external.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<OpenbookV2Market>(),
    )]
    pub openbook_v2_market: AccountLoader<'info, OpenbookV2Market>,

    #[account(
        init,
        // block using the same market index twice
        seeds = [b"OpenbookV2Index".as_ref(), group.key().as_ref(), &market_index.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<OpenbookV2MarketIndexReservation>(),
    )]
    pub index_reservation: AccountLoader<'info, OpenbookV2MarketIndexReservation>,

    #[account(has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
