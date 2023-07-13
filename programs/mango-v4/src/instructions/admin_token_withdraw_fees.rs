use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::{accounts_ix::*, group_seeds};

pub fn admin_token_withdraw_fees(ctx: Context<AdminTokenWithdrawFees>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let group_seeds = group_seeds!(group);
    let fees = bank.collected_fees_native.floor().to_num::<u64>() - bank.fees_withdrawn;
    let amount = fees.min(ctx.accounts.vault.amount);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    bank.fees_withdrawn += amount;

    Ok(())
}
