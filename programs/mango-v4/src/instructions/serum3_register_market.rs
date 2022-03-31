use anchor_lang::prelude::*;

use crate::error::MangoError;
use crate::serum3_cpi::{load_market_state, pubkey_from_u64_array};
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3RegisterMarket<'info> {
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
        seeds = [group.key().as_ref(), b"Serum3Market".as_ref(), serum_market_external.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Serum3Market>(),
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,

    #[account(has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// TODO: should this be "configure_serum_market", which allows reconfiguring?
pub fn serum3_register_market(
    ctx: Context<Serum3RegisterMarket>,
    market_index: Serum3MarketIndex,
) -> Result<()> {
    // TODO: must guard against accidentally using the same market_index twice!

    let base_bank = ctx.accounts.base_bank.load()?;
    let quote_bank = ctx.accounts.quote_bank.load()?;
    let market_external = load_market_state(
        &ctx.accounts.serum_market_external,
        &ctx.accounts.serum_program.key(),
    )?;
    require!(
        pubkey_from_u64_array(market_external.pc_mint) == quote_bank.mint,
        MangoError::SomeError
    );
    require!(
        pubkey_from_u64_array(market_external.coin_mint) == base_bank.mint,
        MangoError::SomeError
    );

    let mut serum_market = ctx.accounts.serum_market.load_init()?;
    *serum_market = Serum3Market {
        group: ctx.accounts.group.key(),
        serum_program: ctx.accounts.serum_program.key(),
        serum_market_external: ctx.accounts.serum_market_external.key(),
        market_index,
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        bump: *ctx.bumps.get("serum_market").ok_or(MangoError::SomeError)?,
        reserved: Default::default(),
    };

    Ok(())
}
