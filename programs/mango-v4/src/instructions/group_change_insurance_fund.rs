use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::{accounts_ix::GroupChangeInsuranceFund, group_seeds};

pub fn group_change_insurance_fund(ctx: Context<GroupChangeInsuranceFund>) -> Result<()> {
    {
        let group = ctx.accounts.group.load()?;
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            ctx.accounts.insurance_vault.amount,
        )?;
        token::close_account(ctx.accounts.close_ctx().with_signer(&[group_seeds]))?;
    }

    {
        let mut group = ctx.accounts.group.load_mut()?;
        group.insurance_vault = ctx.accounts.new_insurance_vault.key();
        group.insurance_mint = ctx.accounts.new_insurance_mint.key();
    }

    Ok(())
}
