use anchor_lang::prelude::*;
use anchor_spl::token;
use fixed::types::I80F48;

use crate::{accounts_ix::*, group_seeds};

pub fn dao_withdraw_fees_token(ctx: Context<DaoWithdrawFeesToken>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let group_seeds = group_seeds!(group);
    let fees = bank.collected_fees_native.floor().to_num::<u64>() - bank.fees_withdrawn_to_dao;
    let amount = fees.min(ctx.accounts.vault.amount);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    bank.fees_withdrawn_to_dao += amount;

    Ok(())
}
