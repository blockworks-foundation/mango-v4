use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::{accounts_ix::*, group_seeds};

pub fn admin_perp_withdraw_fees(ctx: Context<AdminPerpWithdrawFees>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    let group_seeds = group_seeds!(group);
    let fees = perp_market.fees_settled.floor().to_num::<u64>() - perp_market.fees_withdrawn;
    let amount = fees.min(ctx.accounts.vault.amount);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    perp_market.fees_withdrawn += amount;

    Ok(())
}
