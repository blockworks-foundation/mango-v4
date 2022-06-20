use crate::state::*;
use anchor_lang::prelude::*;
use fixed::types::I80F48;

#[derive(Accounts)]
pub struct ComputeHealth<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
}

pub fn compute_health(ctx: Context<ComputeHealth>, health_type: HealthType) -> Result<I80F48> {
    let account = ctx.accounts.account.load()?;
    let health = compute_health_from_fixed_accounts(&account, health_type, ctx.remaining_accounts)?;
    msg!("health: {}", health);

    Ok(health)
}
