use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct CreateSerumOpenOrders<'info> {
    // TODO: do we even need the group?
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, SerumMarket>,

    // TODO: limit?
    pub serum_program: UncheckedAccount<'info>,
    pub serum_market_external: UncheckedAccount<'info>,

    // initialized by this instruction via cpi to serum
    #[account(
        init,
        seeds = [account.key().as_ref(), b"serumoo".as_ref(), serum_market.key().as_ref()],
        bump,
        payer = payer,
        // 12 is the padding serum uses for accounts ("serum" prefix, "padding" postfix)
        space = 12 + std::mem::size_of::<serum_dex::state::OpenOrders>(),
    )]
    pub open_orders: UncheckedAccount<'info>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    //pub rent: Sysvar<'info, Rent>,
}

pub fn create_serum_open_orders(_ctx: Context<CreateSerumOpenOrders>) -> Result<()> {
    // TODO: Call serum_dex::instruction::MarketInstruction::InitOpenOrders

    Ok(())
}
