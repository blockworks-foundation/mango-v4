use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Serum3CreateOpenOrders<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_program: UncheckedAccount<'info>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_market_external: UncheckedAccount<'info>,

    // initialized by this instruction via cpi to serum
    #[account(
        init,
        seeds = [b"Serum3OO".as_ref(), account.key().as_ref(), serum_market.key().as_ref()],
        bump,
        payer = payer,
        owner = serum_program.key(),
        // 12 is the padding serum uses for accounts ("serum" prefix, "padding" postfix)
        space = 12 + std::mem::size_of::<serum_dex::state::OpenOrders>(),
    )]
    /// CHECK: Newly created by serum cpi call
    pub open_orders: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn serum3_create_open_orders(ctx: Context<Serum3CreateOpenOrders>) -> Result<()> {
    cpi_init_open_orders(ctx.accounts)?;

    let serum_market = ctx.accounts.serum_market.load()?;

    let mut account = ctx.accounts.account.load_mut()?;
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let serum_account = account.serum3_create(serum_market.market_index)?;
    serum_account.open_orders = ctx.accounts.open_orders.key();
    serum_account.base_token_index = serum_market.base_token_index;
    serum_account.quote_token_index = serum_market.quote_token_index;

    // Make it so that the token_account_map for the base and quote currency
    // stay permanently blocked. Otherwise users may end up in situations where
    // they can't settle a market because they don't have free token_account_map!
    let (quote_position, _, _) = account.token_get_mut_or_create(serum_market.quote_token_index)?;
    quote_position.in_use_count += 1;
    let (base_position, _, _) = account.token_get_mut_or_create(serum_market.base_token_index)?;
    base_position.in_use_count += 1;

    Ok(())
}

fn cpi_init_open_orders(ctx: &Serum3CreateOpenOrders) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::InitOpenOrders {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
        rent: ctx.rent.to_account_info(),
    }
    .call(&group)
}
