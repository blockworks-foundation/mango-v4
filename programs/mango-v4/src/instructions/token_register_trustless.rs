use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::instructions::INDEX_START;
use crate::state::*;
use crate::util::fill_from_str;

use crate::logs::TokenMetaDataLog;

use crate::accounts_ix::*;

#[allow(clippy::too_many_arguments)]
pub fn token_register_trustless(
    ctx: Context<TokenRegisterTrustless>,
    token_index: TokenIndex,
    name: String,
) -> Result<()> {
    require_neq!(token_index, QUOTE_TOKEN_INDEX);
    require_neq!(token_index, TokenIndex::MAX);

    let net_borrow_limit_window_size_ts = 24 * 60 * 60u64;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        group: ctx.accounts.group.key(),
        name: fill_from_str(&name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config: OracleConfig {
            conf_filter: I80F48::from_num(0.10),
            max_staleness_slots: 10000,
            reserved: [0; 72],
        },
        stable_price_model: StablePriceModel::default(),
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        indexed_deposits: I80F48::ZERO,
        indexed_borrows: I80F48::ZERO,
        index_last_updated: now_ts,
        bank_rate_last_updated: now_ts,
        avg_utilization: I80F48::ZERO,
        // 10% daily adjustment at 0% or 100% utilization
        adjustment_factor: I80F48::from_num(0.004),
        util0: I80F48::from_num(0.5),
        rate0: I80F48::from_num(0.018),
        util1: I80F48::from_num(0.75),
        rate1: I80F48::from_num(0.05),
        max_rate: I80F48::from_num(0.5),
        collected_fees_native: I80F48::ZERO,
        loan_origination_fee_rate: I80F48::from_num(0.0020),
        loan_fee_rate: I80F48::from_num(0.005),
        maint_asset_weight: I80F48::from_num(0.75), // 4x leverage
        init_asset_weight: I80F48::from_num(0.5),   // 2x leverage
        maint_liab_weight: I80F48::from_num(1.25),  // 4x leverage
        init_liab_weight: I80F48::from_num(1.5),    // 2x leverage
        liquidation_fee: I80F48::from_num(0.125),
        dust: I80F48::ZERO,
        flash_loan_token_account_initial: u64::MAX,
        flash_loan_approved_amount: 0,
        token_index,
        bump: *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        min_vault_to_deposits_ratio: 0.2,
        net_borrow_limit_window_size_ts,
        last_net_borrows_window_start_ts: now_ts / net_borrow_limit_window_size_ts
            * net_borrow_limit_window_size_ts,
        net_borrow_limit_per_window_quote: 5_000_000_000, // $5k
        net_borrows_in_window: 0,
        borrow_weight_scale_start_quote: 5_000_000_000.0, // $5k
        deposit_weight_scale_start_quote: 5_000_000_000.0, // $5k
        reduce_only: 0,                                   // allow both deposits and borrows
        force_close: 0,
        padding: Default::default(),
        fees_withdrawn: 0,
        token_conditional_swap_taker_fee_rate: 0.0005,
        token_conditional_swap_maker_fee_rate: 0.0005,
        flash_loan_swap_fee_rate: 0.0005,
        reserved: [0; 2092],
    };
    require_gt!(bank.max_rate, MINIMUM_MAX_RATE);

    if let Ok(oracle_price) =
        bank.oracle_price(&AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?, None)
    {
        bank.stable_price_model
            .reset_to_price(oracle_price.to_num(), now_ts);
    } else {
        bank.stable_price_model.reset_on_nonzero_price = 1;
    }

    bank.verify()?;

    let mut mint_info = ctx.accounts.mint_info.load_init()?;
    *mint_info = MintInfo {
        group: ctx.accounts.group.key(),
        token_index,
        group_insurance_fund: 0,
        padding1: Default::default(),
        mint: ctx.accounts.mint.key(),
        banks: Default::default(),
        vaults: Default::default(),
        oracle: ctx.accounts.oracle.key(),
        registration_time: Clock::get()?.unix_timestamp.try_into().unwrap(),
        reserved: [0; 2560],
    };

    mint_info.banks[0] = ctx.accounts.bank.key();
    mint_info.vaults[0] = ctx.accounts.vault.key();

    emit!(TokenMetaDataLog {
        mango_group: ctx.accounts.group.key(),
        mint: ctx.accounts.mint.key(),
        token_index,
        mint_decimals: ctx.accounts.mint.decimals,
        oracle: ctx.accounts.oracle.key(),
        mint_info: ctx.accounts.mint_info.key(),
    });

    Ok(())
}
