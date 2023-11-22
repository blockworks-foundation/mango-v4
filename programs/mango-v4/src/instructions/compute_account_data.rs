use crate::accounts_ix::*;
use crate::{error::MangoError, events::MangoAccountData, health::*, state::*};
use anchor_lang::prelude::*;

pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
    let group_pk = ctx.accounts.group.key();

    // Avoid people depending on this instruction
    let group = ctx.accounts.group.load()?;
    require!(group.is_testing(), MangoError::SomeError);

    let account = ctx.accounts.account.load_full()?;

    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &group_pk)?;

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let health_cache = new_health_cache(&account.borrow(), &account_retriever, now_ts)?;
    let init_health = health_cache.health(HealthType::Init);
    let maint_health = health_cache.health(HealthType::Maint);

    let equity = compute_equity(&account.borrow(), &account_retriever)?;

    // Potentially too big for the stack!
    emit!(MangoAccountData {
        init_health,
        maint_health,
        equity,
    });

    Ok(())
}
