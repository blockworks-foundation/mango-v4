use anchor_lang::prelude::*;

use fixed::types::I80F48;

use super::InterestRateParams;
use crate::accounts_zerocopy::{AccountInfoRef, LoadMutZeroCopyRef};

use crate::state::*;

use crate::logs::TokenMetaDataLog;

/// Changes a token's parameters.
///
/// In addition to these accounts, all banks must be passed as remaining_accounts
/// in MintInfo order.
#[derive(Accounts)]
pub struct TokenEdit<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,
}

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn token_edit(
    ctx: Context<TokenEdit>,
    oracle_opt: Option<Pubkey>,
    oracle_config_opt: Option<OracleConfigParams>,
    group_insurance_fund_opt: Option<bool>,
    interest_rate_params_opt: Option<InterestRateParams>,
    loan_fee_rate_opt: Option<f32>,
    loan_origination_fee_rate_opt: Option<f32>,
    maint_asset_weight_opt: Option<f32>,
    init_asset_weight_opt: Option<f32>,
    maint_liab_weight_opt: Option<f32>,
    init_liab_weight_opt: Option<f32>,
    liquidation_fee_opt: Option<f32>,
    stable_price_delay_interval_seconds_opt: Option<u32>,
    stable_price_delay_growth_limit_opt: Option<f32>,
    stable_price_growth_limit_opt: Option<f32>,
    min_vault_to_deposits_ratio_opt: Option<f64>,
    net_borrows_limit_quote_opt: Option<i64>,
    net_borrows_window_size_ts_opt: Option<u64>,
    reset_stable_price: bool,
    reset_net_borrow_limit: bool,
) -> Result<()> {
    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    mint_info.verify_banks_ais(ctx.remaining_accounts)?;

    for ai in ctx.remaining_accounts.iter() {
        let mut bank = ai.load_mut::<Bank>()?;

        // note: unchanged fields are inline, and match exact definition in register_token
        // please maintain, and don't remove, makes it easy to reason about which support admin modification

        // unchanged -
        // name
        // group
        // mint
        // vault

        if let Some(oracle_config) = oracle_config_opt.as_ref() {
            bank.oracle_config = oracle_config.to_oracle_config();
        };
        if let Some(oracle) = oracle_opt {
            bank.oracle = oracle;
            mint_info.oracle = oracle;
        }
        if reset_stable_price {
            require_keys_eq!(bank.oracle, ctx.accounts.oracle.key());
            let oracle_price =
                bank.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?, None)?;
            bank.stable_price_model.reset_to_price(
                oracle_price.to_num(),
                Clock::get()?.unix_timestamp.try_into().unwrap(),
            );
        }

        if let Some(group_insurance_fund) = group_insurance_fund_opt {
            mint_info.group_insurance_fund = if group_insurance_fund { 1 } else { 0 };
        };

        // unchanged -
        // deposit_index
        // borrow_index
        // cached_indexed_total_deposits
        // cached_indexed_total_borrows
        // indexed_deposits
        // indexed_borrows
        // last_updated

        if let Some(ref interest_rate_params) = interest_rate_params_opt {
            // TODO: add a require! verifying relation between the parameters
            bank.adjustment_factor = I80F48::from_num(interest_rate_params.adjustment_factor);
            bank.util0 = I80F48::from_num(interest_rate_params.util0);
            bank.rate0 = I80F48::from_num(interest_rate_params.rate0);
            bank.util1 = I80F48::from_num(interest_rate_params.util1);
            bank.rate1 = I80F48::from_num(interest_rate_params.rate1);
            bank.max_rate = I80F48::from_num(interest_rate_params.max_rate);
        }

        // unchanged -
        // collected_fees_native

        if let Some(loan_origination_fee_rate) = loan_origination_fee_rate_opt {
            bank.loan_origination_fee_rate = I80F48::from_num(loan_origination_fee_rate);
        }
        if let Some(loan_fee_rate) = loan_fee_rate_opt {
            bank.loan_fee_rate = I80F48::from_num(loan_fee_rate);
        }

        if let Some(maint_asset_weight) = maint_asset_weight_opt {
            bank.maint_asset_weight = I80F48::from_num(maint_asset_weight);
        }
        if let Some(init_asset_weight) = init_asset_weight_opt {
            bank.init_asset_weight = I80F48::from_num(init_asset_weight);
        }
        if let Some(maint_liab_weight) = maint_liab_weight_opt {
            bank.maint_liab_weight = I80F48::from_num(maint_liab_weight);
        }
        if let Some(init_liab_weight) = init_liab_weight_opt {
            bank.init_liab_weight = I80F48::from_num(init_liab_weight);
        }
        if let Some(liquidation_fee) = liquidation_fee_opt {
            bank.liquidation_fee = I80F48::from_num(liquidation_fee);
        }

        if let Some(stable_price_delay_interval_seconds) = stable_price_delay_interval_seconds_opt {
            // Updating this makes the old delay values slightly inconsistent
            bank.stable_price_model.delay_interval_seconds = stable_price_delay_interval_seconds;
        }
        if let Some(stable_price_delay_growth_limit) = stable_price_delay_growth_limit_opt {
            bank.stable_price_model.delay_growth_limit = stable_price_delay_growth_limit;
        }
        if let Some(stable_price_growth_limit) = stable_price_growth_limit_opt {
            bank.stable_price_model.stable_growth_limit = stable_price_growth_limit;
        }

        if let Some(min_vault_to_deposits_ratio) = min_vault_to_deposits_ratio_opt {
            bank.min_vault_to_deposits_ratio = min_vault_to_deposits_ratio;
        }
        if let Some(net_borrows_limit_quote) = net_borrows_limit_quote_opt {
            bank.net_borrows_limit_quote = net_borrows_limit_quote;
        }
        if let Some(net_borrows_window_size_ts) = net_borrows_window_size_ts_opt {
            bank.net_borrows_window_size_ts = net_borrows_window_size_ts;
        }
        if reset_net_borrow_limit {
            bank.net_borrows_in_window = 0;
            bank.last_net_borrows_window_start_ts = 0;
        }

        // unchanged -
        // dust
        // flash_loan_token_account_initial
        // flash_loan_approved_amount
        // token_index
        // bump
        // mint_decimals
        // bank_num
        // reserved
    }

    // Assumes that there is at least one bank
    let bank = ctx.remaining_accounts.first().unwrap().load_mut::<Bank>()?;

    emit!(TokenMetaDataLog {
        mango_group: ctx.accounts.group.key(),
        mint: mint_info.mint.key(),
        token_index: bank.token_index,
        mint_decimals: bank.mint_decimals,
        oracle: mint_info.oracle.key(),
        mint_info: ctx.accounts.mint_info.key(),
    });

    Ok(())
}
