use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
pub struct RegisterSerumMarket<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    // TODO: limit?
    pub serum_program: UncheckedAccount<'info>,
    pub serum_market_external: UncheckedAccount<'info>,

    #[account(
        init,
        // using the serum_market_external in the seed guards against registering the same market twice
        seeds = [group.key().as_ref(), b"SerumMarket".as_ref(), serum_market_external.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<SerumMarket>(),
    )]
    pub serum_market: AccountLoader<'info, SerumMarket>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// TODO: should this be "configure_serum_market", which allows reconfiguring?
pub fn register_serum_market(
    ctx: Context<RegisterSerumMarket>,
    market_index: SerumMarketIndex,
    base_token_index: TokenIndex,
    quote_token_index: TokenIndex,
) -> Result<()> {
    // TODO: must guard against accidentally using the same market_index twice!
    // TODO: verify that base_token_index and quote_token_index are correct!

    let mut serum_market = ctx.accounts.serum_market.load_init()?;
    *serum_market = SerumMarket {
        group: ctx.accounts.group.key(),
        serum_program: ctx.accounts.serum_program.key(),
        serum_market_external: ctx.accounts.serum_market_external.key(),
        market_index,
        base_token_index,
        quote_token_index,
        bump: *ctx.bumps.get("serum_market").ok_or(MangoError::SomeError)?,
    };

    Ok(())
}
