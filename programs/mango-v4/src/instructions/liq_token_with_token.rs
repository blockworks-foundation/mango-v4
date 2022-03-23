use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct LiqTokenWithToken<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.owner == liqor_owner.key(),
    )]
    pub liqor: AccountLoader<'info, MangoAccount>,
    pub liqor_owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
    )]
    pub liqee: AccountLoader<'info, MangoAccount>,

    // TODO: these banks are also passed in remainingAccounts
    #[account(
        mut,
        has_one = group,
        constraint = asset_bank.load()?.token_index != liab_bank.load()?.token_index,
    )]
    pub asset_bank: AccountLoader<'info, Bank>,

    #[account(
        mut,
        has_one = group,
    )]
    pub liab_bank: AccountLoader<'info, Bank>,
}

pub fn liq_token_with_token(ctx: Context<LiqTokenWithToken>) -> Result<()> {
    //
    // Health computation
    //
    let liqee = ctx.accounts.liqee.load()?;
    let health = compute_health_by_scanning_accounts(&liqee, ctx.remaining_accounts)?;
    msg!("health: {}", health);

    // TODO: everything

    Ok(())
}
