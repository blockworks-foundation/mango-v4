use anchor_lang::prelude::*;
use anchor_spl::token;
use fixed::types::I80F48;

use crate::{accounts_ix::*, group_seeds};

pub fn dao_withdraw_fees_perp_market(ctx: Context<DaoWithdrawFeesPerpMarket>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    let group_seeds = group_seeds!(group);
    let amount = perp_market
        .fees_settled
        .floor()
        .to_num::<u64>()
        .min(ctx.accounts.vault.amount);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    let amount_i80f48 = I80F48::from(amount);
    perp_market.fees_settled -= amount_i80f48;

    Ok(())
}
