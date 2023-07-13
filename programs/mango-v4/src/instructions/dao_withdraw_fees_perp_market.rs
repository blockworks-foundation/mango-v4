use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::{accounts_ix::*, group_seeds};

pub fn dao_withdraw_fees_perp_market(ctx: Context<DaoWithdrawFeesPerpMarket>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    let group_seeds = group_seeds!(group);
    let fees = perp_market.fees_settled.floor().to_num::<u64>() - perp_market.fees_withdrawn_to_dao;
    let amount = fees.min(ctx.accounts.vault.amount);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    perp_market.fees_withdrawn_to_dao += amount;

    Ok(())
}
