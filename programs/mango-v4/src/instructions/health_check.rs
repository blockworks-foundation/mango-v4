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

pub fn health_check(
    ctx: Context<HealthCheck>,
    min_value: f64,
    health_check_kind: HealthCheckKind,
) -> Result<()> {
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

    let min_value = I80F48::from_num(min_value);
    let actual_value = match health_check_kind {
        HealthCheckKind::Maint => health_cache.health(HealthType::Maint),
        HealthCheckKind::Init => health_cache.health(HealthType::Init),
        HealthCheckKind::LiquidationEnd => health_cache.health(HealthType::LiquidationEnd),
        HealthCheckKind::MaintRatio => health_cache.health_ratio(HealthType::Maint),
        HealthCheckKind::InitRatio => health_cache.health_ratio(HealthType::Init),
        HealthCheckKind::LiquidationEndRatio => {
            health_cache.health_ratio(HealthType::LiquidationEnd)
        }
    };

    // msg!("{}", actual_value);
    require_gte!(actual_value, min_value, MangoError::InvalidHealth);

    Ok(())
}
