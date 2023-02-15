use crate::accounts_ix::*;
use anchor_lang::prelude::*;

pub fn serum3_edit_market(
    ctx: Context<Serum3EditMarket>,
    reduce_only_opt: Option<bool>,
) -> Result<()> {
    let mut serum3_market = ctx.accounts.market.load_mut()?;

    if let Some(reduce_only) = reduce_only_opt {
        msg!(
            "Reduce only: old - {:?}, new - {:?}",
            serum3_market.reduce_only,
            u8::from(reduce_only)
        );
        serum3_market.reduce_only = u8::from(reduce_only);
    };
    Ok(())
}
