use crate::error::MangoError;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(market_index: Serum3MarketIndex)]
pub struct Serum3RegisterMarket<'info> {
    #[account(
        mut,
        constraint = group.load()?.is_ix_enabled(IxGate::Serum3RegisterMarket) @ MangoError::IxIsDisabled,
        constraint = group.load()?.serum3_supported()
    )]
    pub group: AccountLoader<'info, Group>,
    /// group admin or fast listing admin, checked at #1
    pub admin: Signer<'info>,

    /// CHECK: Can register a market for any serum program
    pub serum_program: UncheckedAccount<'info>,
    /// CHECK: Can register any serum market
    pub serum_market_external: UncheckedAccount<'info>,

    #[account(
        init,
        // using the serum_market_external in the seed guards against registering the same market twice
        seeds = [b"Serum3Market".as_ref(), group.key().as_ref(), serum_market_external.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Serum3Market>(),
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    #[account(
        init,
        // block using the same market index twice
        seeds = [b"Serum3Index".as_ref(), group.key().as_ref(), &market_index.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Serum3MarketIndexReservation>(),
    )]
    pub index_reservation: AccountLoader<'info, Serum3MarketIndexReservation>,

    #[account(has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
