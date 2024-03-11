use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::{Contextable, MangoError};
use crate::health::{
    new_fixed_order_account_retriever_with_optional_banks,
    new_health_cache_skipping_missing_banks_and_bad_oracles, HealthType,
};
use crate::state::*;
use crate::util::clock_now;

pub fn health_check(ctx: Context<HealthCheck>, min_health_maintenance_ratio: f64) -> Result<()> {
    let account = ctx.accounts.account.load_full_mut()?;
    let (now_ts, now_slot) = clock_now();

    let retriever = new_fixed_order_account_retriever_with_optional_banks(
        ctx.remaining_accounts,
        &account.borrow(),
        now_slot,
    )?;
    let health_cache = new_health_cache_skipping_missing_banks_and_bad_oracles(
        &account.borrow(),
        &retriever,
        now_ts,
    )
    .context("health_check health cache")?;

    let min_health_maintenance_ratio = I80F48::from_num(min_health_maintenance_ratio);
    let maintenance_ratio = health_cache.health_ratio(HealthType::Maint);

    require_gte!(
        maintenance_ratio,
        min_health_maintenance_ratio,
        MangoError::InvalidHealth
    );

    Ok(())
}
