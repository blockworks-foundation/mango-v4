use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::{accounts_ix::GroupWithdrawInsuranceFund, group_seeds};

pub fn group_withdraw_insurance_fund(
    ctx: Context<GroupWithdrawInsuranceFund>,
    amount: u64,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let group_seeds = group_seeds!(group);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount.min(ctx.accounts.insurance_vault.amount),
    )?;

    Ok(())
}
