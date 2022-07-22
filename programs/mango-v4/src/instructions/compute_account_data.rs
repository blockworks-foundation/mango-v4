use crate::{events::MangoAccountData, state::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ComputeAccountData<'info> {
    pub group: AccountLoader<'info, Group>,

    pub account: UncheckedAccount<'info>,
}

pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
    let group_pk = ctx.accounts.group.key();

    let mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new_init(&ctx.accounts.account)?;
    let account: MangoAccountAcc = mal.load()?;

    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &group_pk)?;

    let health_cache = new_health_cache(&account, &account_retriever)?;
    let init_health = health_cache.health(HealthType::Init);
    let maint_health = health_cache.health(HealthType::Maint);

    let equity = compute_equity(&account, &account_retriever)?;

    emit!(MangoAccountData {
        health_cache,
        init_health,
        maint_health,
        equity,
    });

    Ok(())
}
