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
    OracleConfigParams, PerpMarketIndex, PlaceOrderType, Serum3MarketIndex, Side, TokenIndex,
};

declare_id!("4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg");

#[program]
pub mod mango_v4 {
    use super::*;

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
        )?;
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
        min_vault_to_deposits_ratio: f64,
        net_borrow_limit_window_size_ts: u64,
        net_borrow_limit_per_window_quote: i64,
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
            min_vault_to_deposits_ratio,
            net_borrow_limit_window_size_ts,
            net_borrow_limit_per_window_quote,
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
        reduce_only_opt: Option<bool>,
        name_opt: Option<String>,
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
            ctx,
            account_num,
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
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
        instructions::account_expand(ctx, token_count, serum3_count, perp_count, perp_oo_count)?;
        Ok(())
    }

    pub fn account_edit(
        ctx: Context<AccountEdit>,
        name_opt: Option<String>,
        delegate_opt: Option<Pubkey>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::account_edit(ctx, name_opt, delegate_opt)?;
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
        instructions::flash_loan_begin(ctx, loan_amounts)?;
        Ok(())
    }

    pub fn flash_loan_end<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanEnd<'info>>,
        flash_loan_type: FlashLoanType,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::flash_loan_end(ctx, flash_loan_type)?;
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
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_register_market(ctx, market_index, name)?;
        Ok(())
    }

    pub fn serum3_edit_market(
        ctx: Context<Serum3EditMarket>,
        reduce_only_opt: Option<bool>,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_edit_market(ctx, reduce_only_opt)?;
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

    pub fn serum3_cancel_all_orders(ctx: Context<Serum3CancelAllOrders>, limit: u8) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_cancel_all_orders(ctx, limit)?;
        Ok(())
    }

    /// Settles all free funds from the OpenOrders account into the MangoAccount.
    ///
    /// Any serum "referrer rebates" (ui fees) are considered Mango fees.
    pub fn serum3_settle_funds(ctx: Context<Serum3SettleFunds>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::serum3_settle_funds(ctx.accounts, None, true)?;
        Ok(())
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

    pub fn token_liq_bankruptcy(
        ctx: Context<TokenLiqBankruptcy>,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::token_liq_bankruptcy(ctx, max_liab_transfer)?;
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
            params: match order_type {
                PlaceOrderType::Market => OrderParams::Market,
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
        #[cfg(feature = "enable-gpl")]
        instructions::perp_liq_negative_pnl_or_bankruptcy(ctx, max_liab_transfer)?;
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

    pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
        #[cfg(feature = "enable-gpl")]
        instructions::compute_account_data(ctx)?;
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
