use crate::{events::MangoAccountData, state::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ComputeAccountData<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
}

pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
    let group_pk = ctx.accounts.group.key();
    let account = ctx.accounts.account.load()?;

    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &group_pk)?;

    let init_health = compute_health(&account, HealthType::Init, &account_retriever)?;
    let maint_health = compute_health(&account, HealthType::Maint, &account_retriever)?;

    let equity = compute_equity(&account, &account_retriever)?;

    emit!(MangoAccountData {
        init_health,
        maint_health,
        equity
    });

    Ok(())
}
