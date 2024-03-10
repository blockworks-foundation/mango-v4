#![allow(unused_variables)]

use fixed::types::I80F48;

#[macro_use]
pub mod util;

extern crate core;
extern crate static_assertions;

use anchor_lang::prelude::*;

use accounts_ix::*;

pub mod accounts_ix;
pub mod accounts_zerocopy;
pub mod address_lookup_table_program;
mod allocator;
pub mod error;
pub mod events;
pub mod health;
pub mod i80f48;
pub mod logs;
pub mod serum3_cpi;
pub mod state;
pub mod types;

#[cfg(feature = "enable-gpl")]
pub mod instructions;

#[cfg(all(not(feature = "no-entrypoint"), not(feature = "enable-gpl")))]

compile_error!("compiling the program entrypoint without 'enable-gpl' makes no sense, enable it or use the 'cpi' or 'client' features");

use state::{
    IxGate, OpenbookV2MarketIndex, OracleConfigParams, PerpMarketIndex, PlaceOrderType,
    SelfTradeBehavior, Serum3MarketIndex, Side, TokenConditionalSwap,
    TokenConditionalSwapDisplayPriceStyle, TokenConditionalSwapIntention, TokenConditionalSwapType,
    TokenIndex, TCS_START_INCENTIVE,
};

declare_id!("4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg");

#[program]
pub mod mango_v4 {
    use super::*;
    use error::*;

    pub fn admin_token_withdraw_fees(ctx: Context<AdminTokenWithdrawFees>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::admin_token_withdraw_fees(ctx)?;
        Ok(())
    }

    pub fn admin_perp_withdraw_fees(ctx: Context<AdminPerpWithdrawFees>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::admin_perp_withdraw_fees(ctx)?;
        Ok(())
    }

    pub fn group_create(
        ctx: Context<GroupCreate>,
        group_num: u32,
        testing: u8,
        version: u8,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::group_create(ctx, group_num, testing, version)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn group_edit(
        ctx: Context<GroupEdit>,
        admin_opt: Option<Pubkey>,
        fast_listing_admin_opt: Option<Pubkey>,
        security_admin_opt: Option<Pubkey>,
        testing_opt: Option<u8>,
        version_opt: Option<u8>,
        deposit_limit_quote_opt: Option<u64>,
        buyback_fees_opt: Option<bool>,
        buyback_fees_bonus_factor_opt: Option<f32>,
        buyback_fees_swap_mango_account_opt: Option<Pubkey>,
        mngo_token_index_opt: Option<TokenIndex>,
        buyback_fees_expiry_interval_opt: Option<u64>,
        allowed_fast_listings_per_interval_opt: Option<u16>,
        collateral_fee_interval_opt: Option<u64>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::group_edit(
            ctx,
            admin_opt,
            fast_listing_admin_opt,
            security_admin_opt,
            testing_opt,
            version_opt,
            deposit_limit_quote_opt,
            buyback_fees_opt,
            buyback_fees_bonus_factor_opt,
            buyback_fees_swap_mango_account_opt,
            mngo_token_index_opt,
            buyback_fees_expiry_interval_opt,
            allowed_fast_listings_per_interval_opt,
            collateral_fee_interval_opt,
        )?;
        Ok(())
    }

    pub fn group_withdraw_insurance_fund(
        ctx: Context<GroupWithdrawInsuranceFund>,
        amount: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::group_withdraw_insurance_fund(ctx, amount)?;
        Ok(())
    }

    pub fn ix_gate_set(ctx: Context<IxGateSet>, ix_gate: u128) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::ix_gate_set(ctx, ix_gate)?;
        Ok(())
    }

    pub fn group_close(ctx: Context<GroupClose>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::group_close(ctx)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn token_register(
        ctx: Context<TokenRegister>,
        token_index: TokenIndex,
        name: String,
        oracle_config: OracleConfigParams,
        interest_rate_params: InterestRateParams,
        loan_fee_rate: f32,
        loan_origination_fee_rate: f32,
        maint_asset_weight: f32,
        init_asset_weight: f32,
        maint_liab_weight: f32,
        init_liab_weight: f32,
        liquidation_fee: f32,
        stable_price_delay_interval_seconds: u32,
        stable_price_delay_growth_limit: f32,
        stable_price_growth_limit: f32,
        min_vault_to_deposits_ratio: f64,
        net_borrow_limit_window_size_ts: u64,
        net_borrow_limit_per_window_quote: i64,
        borrow_weight_scale_start_quote: f64,
        deposit_weight_scale_start_quote: f64,
        reduce_only: u8,
        token_conditional_swap_taker_fee_rate: f32,
        token_conditional_swap_maker_fee_rate: f32,
        flash_loan_swap_fee_rate: f32,
        interest_curve_scaling: f32,
        interest_target_utilization: f32,
        group_insurance_fund: bool,
        deposit_limit: u64,
        zero_util_rate: f32,
        platform_liquidation_fee: f32,
        disable_asset_liquidation: bool,
        collateral_fee_per_day: f32,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_register(
            ctx,
            token_index,
            name,
            oracle_config,
            interest_rate_params,
            loan_fee_rate,
            loan_origination_fee_rate,
            maint_asset_weight,
            init_asset_weight,
            maint_liab_weight,
            init_liab_weight,
            liquidation_fee,
            stable_price_delay_interval_seconds,
            stable_price_delay_growth_limit,
            stable_price_growth_limit,
            min_vault_to_deposits_ratio,
            net_borrow_limit_window_size_ts,
            net_borrow_limit_per_window_quote,
            borrow_weight_scale_start_quote,
            deposit_weight_scale_start_quote,
            reduce_only,
            token_conditional_swap_taker_fee_rate,
            token_conditional_swap_maker_fee_rate,
            flash_loan_swap_fee_rate,
            interest_curve_scaling,
            interest_target_utilization,
            group_insurance_fund,
            deposit_limit,
            zero_util_rate,
            platform_liquidation_fee,
            disable_asset_liquidation,
            collateral_fee_per_day,
        )?;
        Ok(())
    }

    pub fn token_register_trustless(
        ctx: Context<TokenRegisterTrustless>,
        token_index: TokenIndex,
        name: String,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_register_trustless(ctx, token_index, name)?;
        Ok(())
    }

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
        net_borrow_limit_per_window_quote_opt: Option<i64>,
        net_borrow_limit_window_size_ts_opt: Option<u64>,
        borrow_weight_scale_start_quote_opt: Option<f64>,
        deposit_weight_scale_start_quote_opt: Option<f64>,
        reset_stable_price: bool,
        reset_net_borrow_limit: bool,
        reduce_only_opt: Option<u8>,
        name_opt: Option<String>,
        force_close_opt: Option<bool>,
        token_conditional_swap_taker_fee_rate_opt: Option<f32>,
        token_conditional_swap_maker_fee_rate_opt: Option<f32>,
        flash_loan_swap_fee_rate_opt: Option<f32>,
        interest_curve_scaling_opt: Option<f32>,
        interest_target_utilization_opt: Option<f32>,
        maint_weight_shift_start_opt: Option<u64>,
        maint_weight_shift_end_opt: Option<u64>,
        maint_weight_shift_asset_target_opt: Option<f32>,
        maint_weight_shift_liab_target_opt: Option<f32>,
        maint_weight_shift_abort: bool,
        set_fallback_oracle: bool,
        deposit_limit_opt: Option<u64>,
        zero_util_rate_opt: Option<f32>,
        platform_liquidation_fee_opt: Option<f32>,
        disable_asset_liquidation_opt: Option<bool>,
        collateral_fee_per_day_opt: Option<f32>,
        force_withdraw_opt: Option<bool>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_edit(
            ctx,
            oracle_opt,
            oracle_config_opt,
            group_insurance_fund_opt,
            interest_rate_params_opt,
            loan_fee_rate_opt,
            loan_origination_fee_rate_opt,
            maint_asset_weight_opt,
            init_asset_weight_opt,
            maint_liab_weight_opt,
            init_liab_weight_opt,
            liquidation_fee_opt,
            stable_price_delay_interval_seconds_opt,
            stable_price_delay_growth_limit_opt,
            stable_price_growth_limit_opt,
            min_vault_to_deposits_ratio_opt,
            net_borrow_limit_per_window_quote_opt,
            net_borrow_limit_window_size_ts_opt,
            borrow_weight_scale_start_quote_opt,
            deposit_weight_scale_start_quote_opt,
            reset_stable_price,
            reset_net_borrow_limit,
            reduce_only_opt,
            name_opt,
            force_close_opt,
            token_conditional_swap_taker_fee_rate_opt,
            token_conditional_swap_maker_fee_rate_opt,
            flash_loan_swap_fee_rate_opt,
            interest_curve_scaling_opt,
            interest_target_utilization_opt,
            maint_weight_shift_start_opt,
            maint_weight_shift_end_opt,
            maint_weight_shift_asset_target_opt,
            maint_weight_shift_liab_target_opt,
            maint_weight_shift_abort,
            set_fallback_oracle,
            deposit_limit_opt,
            zero_util_rate_opt,
            platform_liquidation_fee_opt,
            disable_asset_liquidation_opt,
            collateral_fee_per_day_opt,
            force_withdraw_opt,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn token_add_bank(
        ctx: Context<TokenAddBank>,
        token_index: TokenIndex,
        bank_num: u32,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_add_bank(ctx, token_index, bank_num)?;
        Ok(())
    }

    pub fn token_deregister<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_deregister(ctx)?;
        Ok(())
    }

    pub fn token_update_index_and_rate(ctx: Context<TokenUpdateIndexAndRate>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_update_index_and_rate(ctx)?;
        Ok(())
    }

    pub fn account_create(
        ctx: Context<AccountCreate>,
        account_num: u32,
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
        name: String,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_create(
            &ctx.accounts.account,
            *ctx.bumps.get("account").ok_or(MangoError::SomeError)?,
            ctx.accounts.group.key(),
            ctx.accounts.owner.key(),
            account_num,
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
            0,
            name,
        )?;
        Ok(())
    }

    pub fn account_create_v2(
        ctx: Context<AccountCreateV2>,
        account_num: u32,
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
        token_conditional_swap_count: u8,
        name: String,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_create(
            &ctx.accounts.account,
            *ctx.bumps.get("account").ok_or(MangoError::SomeError)?,
            ctx.accounts.group.key(),
            ctx.accounts.owner.key(),
            account_num,
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
            token_conditional_swap_count,
            name,
        )?;
        Ok(())
    }

    pub fn account_expand(
        ctx: Context<AccountExpand>,
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_expand(ctx, token_count, serum3_count, perp_count, perp_oo_count, 0)?;
        Ok(())
    }

    pub fn account_expand_v2(
        ctx: Context<AccountExpand>,
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
        token_conditional_swap_count: u8,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_expand(
            ctx,
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
            token_conditional_swap_count,
        )?;
        Ok(())
    }

    pub fn account_size_migration(ctx: Context<AccountSizeMigration>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_size_migration(ctx)?;
        Ok(())
    }

    pub fn account_edit(
        ctx: Context<AccountEdit>,
        name_opt: Option<String>,
        delegate_opt: Option<Pubkey>,
        temporary_delegate_opt: Option<Pubkey>,
        temporary_delegate_expiry_opt: Option<u64>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_edit(
            ctx,
            name_opt,
            delegate_opt,
            temporary_delegate_opt,
            temporary_delegate_expiry_opt,
        )?;
        Ok(())
    }

    pub fn account_toggle_freeze(ctx: Context<AccountToggleFreeze>, freeze: bool) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_toggle_freeze(ctx, freeze)?;
        Ok(())
    }

    pub fn account_close(ctx: Context<AccountClose>, force_close: bool) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_close(ctx, force_close)?;
        Ok(())
    }

    pub fn account_buyback_fees_with_mngo(
        ctx: Context<AccountBuybackFeesWithMngo>,
        max_buyback_usd: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_buyback_fees_with_mngo(ctx, max_buyback_usd)?;
        Ok(())
    }

    // todo:
    // ckamm: generally, using an I80F48 arg will make it harder to call
    // because generic anchor clients won't know how to deal with it
    // and it's tricky to use in typescript generally
    // lets do an interface pass later
    pub fn stub_oracle_create(ctx: Context<StubOracleCreate>, price: I80F48) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::stub_oracle_create(ctx, price)?;
        Ok(())
    }

    pub fn stub_oracle_close(ctx: Context<StubOracleClose>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::stub_oracle_close(ctx)?;
        Ok(())
    }

    pub fn stub_oracle_set(ctx: Context<StubOracleSet>, price: I80F48) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::stub_oracle_set(ctx, price)?;
        Ok(())
    }

    pub fn stub_oracle_set_test(
        ctx: Context<StubOracleSet>,
        price: I80F48,
        last_update_slot: u64,
        deviation: I80F48,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::stub_oracle_set_test(ctx, price, last_update_slot, deviation)?;
        Ok(())
    }

    pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64, reduce_only: bool) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_deposit(ctx, amount, reduce_only)?;
        Ok(())
    }

    pub fn token_deposit_into_existing(
        ctx: Context<TokenDepositIntoExisting>,
        amount: u64,
        reduce_only: bool,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_deposit_into_existing(ctx, amount, reduce_only)?;
        Ok(())
    }

    pub fn token_withdraw(
        ctx: Context<TokenWithdraw>,
        amount: u64,
        allow_borrow: bool,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_withdraw(ctx, amount, allow_borrow)?;
        Ok(())
    }

    pub fn flash_loan_begin<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanBegin<'info>>,
        loan_amounts: Vec<u64>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::flash_loan_begin(
            ctx.program_id,
            &ctx.accounts.account,
            ctx.accounts.owner.key,
            &ctx.accounts.instructions,
            &ctx.accounts.token_program,
            ctx.remaining_accounts,
            loan_amounts,
        )?;
        Ok(())
    }

    /// A version of flash_loan_begin that's specialized for swaps and needs fewer
    /// bytes in the transaction
    pub fn flash_loan_swap_begin<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanSwapBegin<'info>>,
        loan_amount: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::flash_loan_swap_begin(ctx, loan_amount)?;
        Ok(())
    }

    pub fn flash_loan_end<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanEnd<'info>>,
        flash_loan_type: FlashLoanType,
    ) -> Result<()> {
        Err(error_msg!("FlashLoanEnd was replaced by FlashLoanEndV2"))
    }

    pub fn flash_loan_end_v2<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanEnd<'info>>,
        num_loans: u8,
        flash_loan_type: FlashLoanType,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::flash_loan_end(ctx, num_loans, flash_loan_type)?;
        Ok(())
    }

    pub fn health_region_begin<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionBegin<'info>>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::health_region_begin(ctx)?;
        Ok(())
    }

    pub fn health_region_end<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionEnd<'info>>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::health_region_end(ctx)?;
        Ok(())
    }

    ///
    /// Serum
    ///

    pub fn serum3_register_market(
        ctx: Context<Serum3RegisterMarket>,
        market_index: Serum3MarketIndex,
        name: String,
        oracle_price_band: f32,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_register_market(ctx, market_index, name, oracle_price_band)?;
        Ok(())
    }

    pub fn serum3_edit_market(
        ctx: Context<Serum3EditMarket>,
        reduce_only_opt: Option<bool>,
        force_close_opt: Option<bool>,
        name_opt: Option<String>,
        oracle_price_band_opt: Option<f32>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_edit_market(
            ctx,
            reduce_only_opt,
            force_close_opt,
            name_opt,
            oracle_price_band_opt,
        )?;
        Ok(())
    }

    // note:
    // pub fn serum3_edit_market - doesn't exist since a mango serum3 market only contains the properties
    // registered base and quote token pairs, and serum3 external market its pointing to, and none of them
    // should be edited once set on creation

    pub fn serum3_deregister_market(ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_deregister_market(ctx)?;
        Ok(())
    }

    pub fn serum3_create_open_orders(ctx: Context<Serum3CreateOpenOrders>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_create_open_orders(ctx)?;
        Ok(())
    }

    pub fn serum3_close_open_orders(ctx: Context<Serum3CloseOpenOrders>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_close_open_orders(ctx)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn serum3_place_order(
        ctx: Context<Serum3PlaceOrder>,
        side: Serum3Side,
        limit_price: u64,
        max_base_qty: u64,
        max_native_quote_qty_including_fees: u64,
        self_trade_behavior: Serum3SelfTradeBehavior,
        order_type: Serum3OrderType,
        client_order_id: u64,
        limit: u16,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_place_order(
            ctx,
            side,
            limit_price,
            max_base_qty,
            max_native_quote_qty_including_fees,
            self_trade_behavior,
            order_type,
            client_order_id,
            limit,
            false,
        )?;
        Ok(())
    }

    /// requires the receiver_bank in the health account list to be writable
    #[allow(clippy::too_many_arguments)]
    pub fn serum3_place_order_v2(
        ctx: Context<Serum3PlaceOrder>,
        side: Serum3Side,
        limit_price: u64,
        max_base_qty: u64,
        max_native_quote_qty_including_fees: u64,
        self_trade_behavior: Serum3SelfTradeBehavior,
        order_type: Serum3OrderType,
        client_order_id: u64,
        limit: u16,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_place_order(
            ctx,
            side,
            limit_price,
            max_base_qty,
            max_native_quote_qty_including_fees,
            self_trade_behavior,
            order_type,
            client_order_id,
            limit,
            true,
        )?;
        Ok(())
    }

    pub fn serum3_cancel_order(
        ctx: Context<Serum3CancelOrder>,
        side: Serum3Side,
        order_id: u128,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_cancel_order(ctx, side, order_id)?;
        Ok(())
    }

    pub fn serum3_cancel_order_by_client_order_id(
        ctx: Context<Serum3CancelOrder>,
        client_order_id: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_cancel_order_by_client_order_id(ctx, client_order_id)?;
        Ok(())
    }

    pub fn serum3_cancel_all_orders(ctx: Context<Serum3CancelAllOrders>, limit: u8) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_cancel_all_orders(ctx, limit)?;
        Ok(())
    }

    /// Deprecated instruction that used to settles all free funds from the OpenOrders account
    /// into the MangoAccount.
    ///
    /// Any serum "referrer rebates" (ui fees) are considered Mango fees.
    pub fn serum3_settle_funds(ctx: Context<Serum3SettleFunds>) -> Result<()> {
        Err(error_msg!(
            "Serum3SettleFunds was replaced by Serum3SettleFundsV2"
        ))
    }

    /// Like Serum3SettleFunds, but `fees_to_dao` determines if referrer rebates are considered fees
    /// or are credited to the MangoAccount.
    pub fn serum3_settle_funds_v2(
        ctx: Context<Serum3SettleFundsV2>,
        fees_to_dao: bool,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_settle_funds(
            &mut ctx.accounts.v1,
            Some(&mut ctx.accounts.v2),
            fees_to_dao,
        )?;
        Ok(())
    }

    pub fn serum3_liq_force_cancel_orders(
        ctx: Context<Serum3LiqForceCancelOrders>,
        limit: u8,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_liq_force_cancel_orders(ctx, limit)?;
        Ok(())
    }

    // DEPRECATED: use token_liq_with_token
    pub fn liq_token_with_token(
        ctx: Context<TokenLiqWithToken>,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_liq_with_token(
            ctx,
            asset_token_index,
            liab_token_index,
            max_liab_transfer,
        )?;
        Ok(())
    }

    // DEPRECATED: use token_liq_bankruptcy
    pub fn liq_token_bankruptcy(
        ctx: Context<TokenLiqBankruptcy>,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_liq_bankruptcy(ctx, max_liab_transfer)?;
        Ok(())
    }

    pub fn token_liq_with_token(
        ctx: Context<TokenLiqWithToken>,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_liq_with_token(
            ctx,
            asset_token_index,
            liab_token_index,
            max_liab_transfer,
        )?;
        Ok(())
    }

    pub fn token_force_close_borrows_with_token(
        ctx: Context<TokenForceCloseBorrowsWithToken>,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_force_close_borrows_with_token(
            ctx,
            asset_token_index,
            liab_token_index,
            max_liab_transfer,
        )?;
        Ok(())
    }

    pub fn token_liq_bankruptcy(
        ctx: Context<TokenLiqBankruptcy>,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_liq_bankruptcy(ctx, max_liab_transfer)?;
        Ok(())
    }

    pub fn token_force_withdraw(ctx: Context<TokenForceWithdraw>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_force_withdraw(ctx)?;
        Ok(())
    }

    ///
    /// Perps
    ///

    #[allow(clippy::too_many_arguments)]
    pub fn perp_create_market(
        ctx: Context<PerpCreateMarket>,
        perp_market_index: PerpMarketIndex,
        name: String,
        oracle_config: OracleConfigParams,
        base_decimals: u8,
        quote_lot_size: i64,
        base_lot_size: i64,
        maint_base_asset_weight: f32,
        init_base_asset_weight: f32,
        maint_base_liab_weight: f32,
        init_base_liab_weight: f32,
        maint_overall_asset_weight: f32,
        init_overall_asset_weight: f32,
        base_liquidation_fee: f32,
        maker_fee: f32,
        taker_fee: f32,
        min_funding: f32,
        max_funding: f32,
        impact_quantity: i64,
        group_insurance_fund: bool,
        fee_penalty: f32,
        settle_fee_flat: f32,
        settle_fee_amount_threshold: f32,
        settle_fee_fraction_low_health: f32,
        settle_token_index: TokenIndex,
        settle_pnl_limit_factor: f32,
        settle_pnl_limit_window_size_ts: u64,
        positive_pnl_liquidation_fee: f32,
        platform_liquidation_fee: f32,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_create_market(
            ctx,
            perp_market_index,
            settle_token_index,
            name,
            oracle_config,
            base_decimals,
            quote_lot_size,
            base_lot_size,
            maint_base_asset_weight,
            init_base_asset_weight,
            maint_base_liab_weight,
            init_base_liab_weight,
            maint_overall_asset_weight,
            init_overall_asset_weight,
            base_liquidation_fee,
            maker_fee,
            taker_fee,
            min_funding,
            max_funding,
            impact_quantity,
            group_insurance_fund,
            fee_penalty,
            settle_fee_flat,
            settle_fee_amount_threshold,
            settle_fee_fraction_low_health,
            settle_pnl_limit_factor,
            settle_pnl_limit_window_size_ts,
            positive_pnl_liquidation_fee,
            platform_liquidation_fee,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perp_edit_market(
        ctx: Context<PerpEditMarket>,
        oracle_opt: Option<Pubkey>,
        oracle_config_opt: Option<OracleConfigParams>,
        base_decimals_opt: Option<u8>,
        maint_base_asset_weight_opt: Option<f32>,
        init_base_asset_weight_opt: Option<f32>,
        maint_base_liab_weight_opt: Option<f32>,
        init_base_liab_weight_opt: Option<f32>,
        maint_overall_asset_weight_opt: Option<f32>,
        init_overall_asset_weight_opt: Option<f32>,
        base_liquidation_fee_opt: Option<f32>,
        maker_fee_opt: Option<f32>,
        taker_fee_opt: Option<f32>,
        min_funding_opt: Option<f32>,
        max_funding_opt: Option<f32>,
        impact_quantity_opt: Option<i64>,
        group_insurance_fund_opt: Option<bool>,
        fee_penalty_opt: Option<f32>,
        settle_fee_flat_opt: Option<f32>,
        settle_fee_amount_threshold_opt: Option<f32>,
        settle_fee_fraction_low_health_opt: Option<f32>,
        stable_price_delay_interval_seconds_opt: Option<u32>,
        stable_price_delay_growth_limit_opt: Option<f32>,
        stable_price_growth_limit_opt: Option<f32>,
        settle_pnl_limit_factor_opt: Option<f32>,
        settle_pnl_limit_window_size_ts_opt: Option<u64>,
        reduce_only_opt: Option<bool>,
        reset_stable_price: bool,
        positive_pnl_liquidation_fee_opt: Option<f32>,
        name_opt: Option<String>,
        force_close_opt: Option<bool>,
        platform_liquidation_fee_opt: Option<f32>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_edit_market(
            ctx,
            oracle_opt,
            oracle_config_opt,
            base_decimals_opt,
            maint_base_asset_weight_opt,
            init_base_asset_weight_opt,
            maint_base_liab_weight_opt,
            init_base_liab_weight_opt,
            maint_overall_asset_weight_opt,
            init_overall_asset_weight_opt,
            base_liquidation_fee_opt,
            maker_fee_opt,
            taker_fee_opt,
            min_funding_opt,
            max_funding_opt,
            impact_quantity_opt,
            group_insurance_fund_opt,
            fee_penalty_opt,
            settle_fee_flat_opt,
            settle_fee_amount_threshold_opt,
            settle_fee_fraction_low_health_opt,
            stable_price_delay_interval_seconds_opt,
            stable_price_delay_growth_limit_opt,
            stable_price_growth_limit_opt,
            settle_pnl_limit_factor_opt,
            settle_pnl_limit_window_size_ts_opt,
            reduce_only_opt,
            reset_stable_price,
            positive_pnl_liquidation_fee_opt,
            name_opt,
            force_close_opt,
            platform_liquidation_fee_opt,
        )?;
        Ok(())
    }

    pub fn perp_close_market(ctx: Context<PerpCloseMarket>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_close_market(ctx)?;
        Ok(())
    }

    pub fn perp_deactivate_position(ctx: Context<PerpDeactivatePosition>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_deactivate_position(ctx)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perp_place_order(
        ctx: Context<PerpPlaceOrder>,
        side: Side,

        // The price in lots (quote lots per base lots)
        // - fill orders on the book up to this price or
        // - place an order on the book at this price.
        // - ignored for Market orders and potentially adjusted for PostOnlySlide orders.
        price_lots: i64,

        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        reduce_only: bool,

        // Timestamp of when order expires
        //
        // Send 0 if you want the order to never expire.
        // Timestamps in the past mean the instruction is skipped.
        // Timestamps in the future are reduced to now + 65535s.
        expiry_timestamp: u64,

        // Maximum number of orders from the book to fill.
        //
        // Use this to limit compute used during order matching.
        // When the limit is reached, processing stops and the instruction succeeds.
        limit: u8,
    ) -> Result<Option<u128>> {
        require_gte!(price_lots, 0);

        use crate::state::{Order, OrderParams};
        let time_in_force = match Order::tif_from_expiry(expiry_timestamp) {
            Some(t) => t,
            None => {
                msg!("Order is already expired");
                return Ok(None);
            }
        };
        let order = Order {
            side,
            max_base_lots,
            max_quote_lots,
            client_order_id,
            reduce_only,
            time_in_force,
            self_trade_behavior: SelfTradeBehavior::default(),
            params: match order_type {
                PlaceOrderType::Market => OrderParams::Market {},
                PlaceOrderType::ImmediateOrCancel => OrderParams::ImmediateOrCancel { price_lots },
                _ => OrderParams::Fixed {
                    price_lots,
                    order_type: order_type.to_post_order_type()?,
                },
            },
        };
        #[cfg(feature = "enable-gpl")]
        return instructions::perp_place_order(ctx, order, limit);

        #[cfg(not(feature = "enable-gpl"))]
        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perp_place_order_v2(
        ctx: Context<PerpPlaceOrder>,
        side: Side,

        // The price in lots (quote lots per base lots)
        // - fill orders on the book up to this price or
        // - place an order on the book at this price.
        // - ignored for Market orders and potentially adjusted for PostOnlySlide orders.
        price_lots: i64,

        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        self_trade_behavior: SelfTradeBehavior,
        reduce_only: bool,

        // Timestamp of when order expires
        //
        // Send 0 if you want the order to never expire.
        // Timestamps in the past mean the instruction is skipped.
        // Timestamps in the future are reduced to now + 65535s.
        expiry_timestamp: u64,

        // Maximum number of orders from the book to fill.
        //
        // Use this to limit compute used during order matching.
        // When the limit is reached, processing stops and the instruction succeeds.
        limit: u8,
    ) -> Result<Option<u128>> {
        require_gte!(price_lots, 0);

        use crate::state::{Order, OrderParams};
        let time_in_force = match Order::tif_from_expiry(expiry_timestamp) {
            Some(t) => t,
            None => {
                msg!("Order is already expired");
                return Ok(None);
            }
        };
        let order = Order {
            side,
            max_base_lots,
            max_quote_lots,
            client_order_id,
            reduce_only,
            time_in_force,
            self_trade_behavior,
            params: match order_type {
                PlaceOrderType::Market => OrderParams::Market {},
                PlaceOrderType::ImmediateOrCancel => OrderParams::ImmediateOrCancel { price_lots },
                _ => OrderParams::Fixed {
                    price_lots,
                    order_type: order_type.to_post_order_type()?,
                },
            },
        };
        #[cfg(feature = "enable-gpl")]
        return instructions::perp_place_order(ctx, order, limit);

        #[cfg(not(feature = "enable-gpl"))]
        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perp_place_order_pegged(
        ctx: Context<PerpPlaceOrder>,
        side: Side,

        // The adjustment from the oracle price, in lots (quote lots per base lots).
        // Orders on the book may be filled at oracle + adjustment (depends on order type).
        price_offset_lots: i64,

        // The limit at which the pegged order shall expire.
        // May be -1 to denote no peg limit.
        //
        // Example: An bid pegged to -20 with peg_limit 100 would expire if the oracle hits 121.
        peg_limit: i64,

        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        reduce_only: bool,

        // Timestamp of when order expires
        //
        // Send 0 if you want the order to never expire.
        // Timestamps in the past mean the instruction is skipped.
        // Timestamps in the future are reduced to now + 65535s.
        expiry_timestamp: u64,

        // Maximum number of orders from the book to fill.
        //
        // Use this to limit compute used during order matching.
        // When the limit is reached, processing stops and the instruction succeeds.
        limit: u8,

        // Oracle staleness limit, in slots. Set to -1 to disable.
        //
        // WARNING: Not currently implemented.
        max_oracle_staleness_slots: i32,
    ) -> Result<Option<u128>> {
        require_gte!(peg_limit, -1);
        require_eq!(max_oracle_staleness_slots, -1); // unimplemented

        use crate::state::{Order, OrderParams};
        let time_in_force = match Order::tif_from_expiry(expiry_timestamp) {
            Some(t) => t,
            None => {
                msg!("Order is already expired");
                return Ok(None);
            }
        };
        let order = Order {
            side,
            max_base_lots,
            max_quote_lots,
            client_order_id,
            reduce_only,
            time_in_force,
            self_trade_behavior: SelfTradeBehavior::DecrementTake,
            params: OrderParams::OraclePegged {
                price_offset_lots,
                order_type: order_type.to_post_order_type()?,
                peg_limit,
                max_oracle_staleness_slots,
            },
        };
        #[cfg(feature = "enable-gpl")]
        return instructions::perp_place_order(ctx, order, limit);

        #[cfg(not(feature = "enable-gpl"))]
        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perp_place_order_pegged_v2(
        ctx: Context<PerpPlaceOrder>,
        side: Side,

        // The adjustment from the oracle price, in lots (quote lots per base lots).
        // Orders on the book may be filled at oracle + adjustment (depends on order type).
        price_offset_lots: i64,

        // The limit at which the pegged order shall expire.
        // May be -1 to denote no peg limit.
        //
        // Example: An bid pegged to -20 with peg_limit 100 would expire if the oracle hits 121.
        peg_limit: i64,

        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        self_trade_behavior: SelfTradeBehavior,
        reduce_only: bool,

        // Timestamp of when order expires
        //
        // Send 0 if you want the order to never expire.
        // Timestamps in the past mean the instruction is skipped.
        // Timestamps in the future are reduced to now + 65535s.
        expiry_timestamp: u64,

        // Maximum number of orders from the book to fill.
        //
        // Use this to limit compute used during order matching.
        // When the limit is reached, processing stops and the instruction succeeds.
        limit: u8,

        // Oracle staleness limit, in slots. Set to -1 to disable.
        //
        // WARNING: Not currently implemented.
        max_oracle_staleness_slots: i32,
    ) -> Result<Option<u128>> {
        require_gte!(peg_limit, -1);
        require_eq!(max_oracle_staleness_slots, -1); // unimplemented

        use crate::state::{Order, OrderParams};
        let time_in_force = match Order::tif_from_expiry(expiry_timestamp) {
            Some(t) => t,
            None => {
                msg!("Order is already expired");
                return Ok(None);
            }
        };
        let order = Order {
            side,
            max_base_lots,
            max_quote_lots,
            client_order_id,
            reduce_only,
            time_in_force,
            self_trade_behavior,
            params: OrderParams::OraclePegged {
                price_offset_lots,
                order_type: order_type.to_post_order_type()?,
                peg_limit,
                max_oracle_staleness_slots,
            },
        };
        #[cfg(feature = "enable-gpl")]
        return instructions::perp_place_order(ctx, order, limit);

        #[cfg(not(feature = "enable-gpl"))]
        Ok(None)
    }

    pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: u128) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_cancel_order(ctx, order_id)?;
        Ok(())
    }

    pub fn perp_cancel_order_by_client_order_id(
        ctx: Context<PerpCancelOrderByClientOrderId>,
        client_order_id: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_cancel_order_by_client_order_id(ctx, client_order_id)?;
        Ok(())
    }

    pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_cancel_all_orders(ctx, limit)?;
        Ok(())
    }

    pub fn perp_cancel_all_orders_by_side(
        ctx: Context<PerpCancelAllOrdersBySide>,
        side_option: Option<Side>,
        limit: u8,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_cancel_all_orders_by_side(ctx, side_option, limit)?;
        Ok(())
    }

    pub fn perp_consume_events(ctx: Context<PerpConsumeEvents>, limit: usize) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_consume_events(ctx, limit)?;
        Ok(())
    }

    pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_update_funding(ctx)?;
        Ok(())
    }

    pub fn perp_settle_pnl(ctx: Context<PerpSettlePnl>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_settle_pnl(ctx)?;
        Ok(())
    }

    pub fn perp_force_close_position(ctx: Context<PerpForceClosePosition>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_force_close_position(ctx)?;
        Ok(())
    }

    pub fn perp_settle_fees(ctx: Context<PerpSettleFees>, max_settle_amount: u64) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_settle_fees(ctx, max_settle_amount)?;
        Ok(())
    }

    pub fn perp_liq_base_or_positive_pnl(
        ctx: Context<PerpLiqBaseOrPositivePnl>,
        max_base_transfer: i64,
        max_pnl_transfer: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_liq_base_or_positive_pnl(ctx, max_base_transfer, max_pnl_transfer)?;
        Ok(())
    }

    pub fn perp_liq_force_cancel_orders(
        ctx: Context<PerpLiqForceCancelOrders>,
        limit: u8,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_liq_force_cancel_orders(ctx, limit)?;
        Ok(())
    }

    pub fn perp_liq_negative_pnl_or_bankruptcy(
        ctx: Context<PerpLiqNegativePnlOrBankruptcy>,
        max_liab_transfer: u64,
    ) -> Result<()> {
        Err(error_msg!(
            "PerpLiqNegativePnlOrBankruptcy was replaced by PerpLiqNegativePnlOrBankruptcyV2"
        ))
    }

    pub fn perp_liq_negative_pnl_or_bankruptcy_v2(
        ctx: Context<PerpLiqNegativePnlOrBankruptcyV2>,
        max_liab_transfer: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::perp_liq_negative_pnl_or_bankruptcy(ctx, max_liab_transfer)?;
        Ok(())
    }

    pub fn token_conditional_swap_create(
        ctx: Context<TokenConditionalSwapCreate>,
        max_buy: u64,
        max_sell: u64,
        expiry_timestamp: u64,
        price_lower_limit: f64,
        price_upper_limit: f64,
        price_premium_rate: f64,
        allow_creating_deposits: bool,
        allow_creating_borrows: bool,
    ) -> Result<()> {
        token_conditional_swap_create_v2(
            ctx,
            max_buy,
            max_sell,
            expiry_timestamp,
            price_lower_limit,
            price_upper_limit,
            price_premium_rate,
            allow_creating_deposits,
            allow_creating_borrows,
            TokenConditionalSwapDisplayPriceStyle::SellTokenPerBuyToken,
            TokenConditionalSwapIntention::Unknown,
        )
    }

    pub fn token_conditional_swap_create_v2(
        ctx: Context<TokenConditionalSwapCreate>,
        max_buy: u64,
        max_sell: u64,
        expiry_timestamp: u64,
        price_lower_limit: f64,
        price_upper_limit: f64,
        price_premium_rate: f64,
        allow_creating_deposits: bool,
        allow_creating_borrows: bool,
        display_price_style: TokenConditionalSwapDisplayPriceStyle,
        intention: TokenConditionalSwapIntention,
    ) -> Result<()> {
        require!(
            ctx.accounts
                .group
                .load()?
                .is_ix_enabled(IxGate::TokenConditionalSwapCreate),
            MangoError::IxIsDisabled
        );
        let tcs = TokenConditionalSwap {
            id: u64::MAX, // set inside
            max_buy,
            max_sell,
            bought: 0,
            sold: 0,
            expiry_timestamp,
            price_lower_limit,
            price_upper_limit,
            price_premium_rate,
            taker_fee_rate: 0.0, // set inside
            maker_fee_rate: 0.0, // set inside
            buy_token_index: ctx.accounts.buy_bank.load()?.token_index,
            sell_token_index: ctx.accounts.sell_bank.load()?.token_index,
            is_configured: 1,
            allow_creating_deposits: u8::from(allow_creating_deposits),
            allow_creating_borrows: u8::from(allow_creating_borrows),
            display_price_style: display_price_style.into(),
            intention: intention.into(),
            tcs_type: TokenConditionalSwapType::FixedPremium.into(),
            padding: Default::default(),
            start_timestamp: 0,  // not started
            duration_seconds: 0, // duration does not matter for FixedPremium
            reserved: [0; 88],
        };

        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_create(ctx, tcs)?;
        Ok(())
    }

    pub fn token_conditional_swap_create_premium_auction(
        ctx: Context<TokenConditionalSwapCreate>,
        max_buy: u64,
        max_sell: u64,
        expiry_timestamp: u64,
        price_lower_limit: f64,
        price_upper_limit: f64,
        max_price_premium_rate: f64,
        allow_creating_deposits: bool,
        allow_creating_borrows: bool, // TODO: require that this is false?
        display_price_style: TokenConditionalSwapDisplayPriceStyle,
        intention: TokenConditionalSwapIntention,
        duration_seconds: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts
                .group
                .load()?
                .is_ix_enabled(IxGate::TokenConditionalSwapCreatePremiumAuction),
            MangoError::IxIsDisabled
        );
        require_gte!(duration_seconds, 1);
        let tcs = TokenConditionalSwap {
            id: u64::MAX, // set inside
            max_buy,
            max_sell,
            bought: 0,
            sold: 0,
            expiry_timestamp,
            price_lower_limit,
            price_upper_limit,
            price_premium_rate: max_price_premium_rate,
            taker_fee_rate: 0.0, // set inside
            maker_fee_rate: 0.0, // set inside
            buy_token_index: ctx.accounts.buy_bank.load()?.token_index,
            sell_token_index: ctx.accounts.sell_bank.load()?.token_index,
            is_configured: 1,
            allow_creating_deposits: u8::from(allow_creating_deposits),
            allow_creating_borrows: u8::from(allow_creating_borrows),
            display_price_style: display_price_style.into(),
            intention: intention.into(),
            tcs_type: TokenConditionalSwapType::PremiumAuction.into(),
            padding: Default::default(),
            start_timestamp: 0, // not started
            duration_seconds,
            reserved: [0; 88],
        };

        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_create(ctx, tcs)?;
        Ok(())
    }

    pub fn token_conditional_swap_create_linear_auction(
        ctx: Context<TokenConditionalSwapCreate>,
        max_buy: u64,
        max_sell: u64,
        expiry_timestamp: u64,
        price_start: f64,
        price_end: f64,
        allow_creating_deposits: bool,
        allow_creating_borrows: bool,
        display_price_style: TokenConditionalSwapDisplayPriceStyle,
        start_timestamp: u64,
        duration_seconds: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts
                .group
                .load()?
                .is_ix_enabled(IxGate::TokenConditionalSwapCreateLinearAuction),
            MangoError::IxIsDisabled
        );
        require_gte!(duration_seconds, 1);

        let buy_token_price = ctx.accounts.buy_bank.load()?.stable_price().to_num::<f64>();
        let sell_token_price = ctx
            .accounts
            .sell_bank
            .load()?
            .stable_price()
            .to_num::<f64>();
        let max_volume =
            (buy_token_price * max_buy as f64).min(sell_token_price * max_sell as f64) as u64;
        require_gte!(
            max_volume,
            TCS_START_INCENTIVE * 10,
            MangoError::TokenConditionalSwapTooSmallForStartIncentive
        );

        let tcs = TokenConditionalSwap {
            id: u64::MAX, // set inside
            max_buy,
            max_sell,
            bought: 0,
            sold: 0,
            expiry_timestamp,
            price_lower_limit: price_start,
            price_upper_limit: price_end,
            price_premium_rate: 0.0, // ignored for linear auctions
            taker_fee_rate: 0.0,     // set inside
            maker_fee_rate: 0.0,     // set inside
            buy_token_index: ctx.accounts.buy_bank.load()?.token_index,
            sell_token_index: ctx.accounts.sell_bank.load()?.token_index,
            is_configured: 1,
            allow_creating_deposits: u8::from(allow_creating_deposits),
            allow_creating_borrows: u8::from(allow_creating_borrows),
            display_price_style: display_price_style.into(),
            intention: TokenConditionalSwapIntention::Unknown.into(),
            tcs_type: TokenConditionalSwapType::LinearAuction.into(),
            padding: Default::default(),
            start_timestamp,
            duration_seconds,
            reserved: [0; 88],
        };

        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_create(ctx, tcs)?;
        Ok(())
    }

    pub fn token_conditional_swap_cancel(
        ctx: Context<TokenConditionalSwapCancel>,
        token_conditional_swap_index: u8,
        token_conditional_swap_id: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_cancel(
            ctx,
            token_conditional_swap_index.into(),
            token_conditional_swap_id,
        )?;
        Ok(())
    }

    // NOTE: It's the triggerer's job to compute liqor_max_* numbers that work with the liqee's health.
    pub fn token_conditional_swap_trigger(
        ctx: Context<TokenConditionalSwapTrigger>,
        token_conditional_swap_index: u8,
        token_conditional_swap_id: u64,
        max_buy_token_to_liqee: u64,
        max_sell_token_to_liqor: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_trigger(
            ctx,
            token_conditional_swap_index.into(),
            token_conditional_swap_id,
            max_buy_token_to_liqee,
            max_sell_token_to_liqor,
            0,
            0.0,
        )?;
        Ok(())
    }

    // NOTE: It's the triggerer's job to compute liqor_max_* numbers that work with the liqee's health.
    pub fn token_conditional_swap_trigger_v2(
        ctx: Context<TokenConditionalSwapTrigger>,
        token_conditional_swap_index: u8,
        token_conditional_swap_id: u64,
        max_buy_token_to_liqee: u64,
        max_sell_token_to_liqor: u64,
        min_buy_token: u64,
        min_taker_price: f32,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_trigger(
            ctx,
            token_conditional_swap_index.into(),
            token_conditional_swap_id,
            max_buy_token_to_liqee,
            max_sell_token_to_liqor,
            min_buy_token,
            min_taker_price as f64,
        )?;
        Ok(())
    }

    pub fn token_conditional_swap_start(
        ctx: Context<TokenConditionalSwapStart>,
        token_conditional_swap_index: u8,
        token_conditional_swap_id: u64,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_conditional_swap_start(
            ctx,
            token_conditional_swap_index.into(),
            token_conditional_swap_id,
        )?;
        Ok(())
    }

    pub fn token_charge_collateral_fees(ctx: Context<TokenChargeCollateralFees>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_charge_collateral_fees(ctx)?;
        Ok(())
    }

    pub fn alt_set(ctx: Context<AltSet>, index: u8) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::alt_set(ctx, index)?;
        Ok(())
    }

    pub fn alt_extend(
        ctx: Context<AltExtend>,
        index: u8,
        new_addresses: Vec<Pubkey>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::alt_extend(ctx, index, new_addresses)?;
        Ok(())
    }

    /// Warning, this instruction is for testing purposes only!
    pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::compute_account_data(ctx)?;
        Ok(())
    }

    ///
    /// OpenbookV2
    ///

    pub fn openbook_v2_register_market(
        ctx: Context<OpenbookV2RegisterMarket>,
        market_index: OpenbookV2MarketIndex,
        name: String,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_edit_market(
        ctx: Context<OpenbookV2EditMarket>,
        reduce_only_opt: Option<bool>,
        force_close_opt: Option<bool>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_deregister_market(ctx: Context<OpenbookV2DeregisterMarket>) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_create_open_orders(
        ctx: Context<OpenbookV2CreateOpenOrders>,
        account_num: u32,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_close_open_orders(ctx: Context<OpenbookV2CloseOpenOrders>) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn openbook_v2_place_order(
        ctx: Context<OpenbookV2PlaceOrder>,
        side: u8, // openbook_v2::state::Side
        limit_price: u64,
        max_base_qty: u64,
        max_native_quote_qty_including_fees: u64,
        self_trade_behavior: u8, // openbook_v2::state::SelfTradeBehavior
        order_type: u8,          // openbook_v2::state::PlaceOrderType
        client_order_id: u64,
        limit: u16,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn openbook_v2_place_taker_order(
        ctx: Context<OpenbookV2PlaceTakeOrder>,
        side: u8, // openbook_v2::state::Side
        limit_price: u64,
        max_base_qty: u64,
        max_native_quote_qty_including_fees: u64,
        self_trade_behavior: u8, // openbook_v2::state::SelfTradeBehavior
        client_order_id: u64,
        limit: u16,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_cancel_order(
        ctx: Context<OpenbookV2CancelOrder>,
        side: u8, // openbook_v2::state::Side
        order_id: u128,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_settle_funds(
        ctx: Context<OpenbookV2SettleFunds>,
        fees_to_dao: bool,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_liq_force_cancel_orders(
        ctx: Context<OpenbookV2LiqForceCancelOrders>,
        limit: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn openbook_v2_cancel_all_orders(
        ctx: Context<OpenbookV2CancelOrder>,
        limit: u8,
    ) -> Result<()> {
        Ok(())
    }

    ///
    /// benchmark
    ///

    pub fn benchmark(ctx: Context<Benchmark>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::benchmark(ctx)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Mango;

impl anchor_lang::Id for Mango {
    fn id() -> Pubkey {
        ID
    }
}

#[cfg(not(feature = "no-entrypoint"))]
use {default_env::default_env, solana_security_txt::security_txt};
#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Mango v4",
    project_url: "https://mango.markets",
    contacts: "email:hello@blockworks.foundation,link:https://docs.mango.markets/mango-markets/bug-bounty,discord:https://discord.gg/mangomarkets",
    policy: "https://github.com/blockworks-foundation/mango-v4/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/blockworks-foundation/mango-v4",
    source_revision: default_env!("GITHUB_SHA", "Unknown source revision"),
    source_release: default_env!("GITHUB_REF_NAME", "Unknown source release")
}
