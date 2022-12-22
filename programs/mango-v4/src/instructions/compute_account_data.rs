use crate::accounts_ix::*;
use crate::{events::MangoAccountData, health::*, state::*};
use anchor_lang::prelude::*;

pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
    let group_pk = ctx.accounts.group.key();

    let account = ctx.accounts.account.load_full()?;

    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &group_pk)?;

    let health_cache = new_health_cache(&account.borrow(), &account_retriever)?;
    let init_health = health_cache.health(HealthType::Init);
    let maint_health = health_cache.health(HealthType::Maint);

    let equity = compute_equity(&account.borrow(), &account_retriever)?;

    emit!(MangoAccountData {
        health_cache,
        init_health,
        maint_health,
        equity,
    });

    Ok(())
}
