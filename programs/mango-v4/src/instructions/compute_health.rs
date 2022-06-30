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
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account)?;
    let health = crate::state::compute_health(&account, health_type, &retriever)?;
    msg!("health: {}", health);

    Ok(health)
}
