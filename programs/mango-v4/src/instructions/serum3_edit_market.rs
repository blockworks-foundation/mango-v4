use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
#[instruction(market_index: Serum3MarketIndex)]
pub struct Serum3EditMarket<'info> {
    #[account(
        has_one = admin,
        constraint = group.load()?.serum3_supported()
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub market: AccountLoader<'info, Serum3Market>,
}

pub fn serum3_edit_market(
    ctx: Context<Serum3EditMarket>,
    reduce_only_opt: Option<bool>,
) -> Result<()> {
    if let Some(reduce_only) = reduce_only_opt {
        msg!(
            "Reduce only: old - {:?}, new - {:?}",
            perp_market.reduce_only,
            u8::from(reduce_only)
        );
        ctx.accounts.market.load_mut()?.reduce_only = u8::from(reduce_only);
    };
    Ok(())
}
