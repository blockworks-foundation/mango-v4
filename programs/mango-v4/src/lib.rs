use fixed::types::I80F48;

#[macro_use]
pub mod util;

extern crate core;
extern crate static_assertions;

use anchor_lang::prelude::*;

use instructions::*;

pub mod accounts_zerocopy;
pub mod address_lookup_table_program;
pub mod error;
pub mod events;
pub mod i80f48;
pub mod instructions;
pub mod logs;
pub mod serum3_cpi;
pub mod state;
pub mod types;

use state::{
    OracleConfigParams, PerpMarketIndex, PlaceOrderType, Serum3MarketIndex, Side, TokenIndex,
};

declare_id!("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");

#[program]
pub mod mango_v4 {
    use super::*;

    pub fn group_create(
        ctx: Context<GroupCreate>,
        group_num: u32,
        testing: u8,
        version: u8,
    ) -> Result<()> {
        instructions::group_create(ctx, group_num, testing, version)
    }

    pub fn group_edit(
        ctx: Context<GroupEdit>,
        admin_opt: Option<Pubkey>,
        fast_listing_admin_opt: Option<Pubkey>,
        testing_opt: Option<u8>,
        version_opt: Option<u8>,
    ) -> Result<()> {
        instructions::group_edit(
            ctx,
            admin_opt,
            fast_listing_admin_opt,
            testing_opt,
            version_opt,
        )
    }

    pub fn group_close(ctx: Context<GroupClose>) -> Result<()> {
        instructions::group_close(ctx)
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
    ) -> Result<()> {
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
        )
    }

    pub fn token_register_trustless(
        ctx: Context<TokenRegisterTrustless>,
        token_index: TokenIndex,
        name: String,
    ) -> Result<()> {
        instructions::token_register_trustless(ctx, token_index, name)
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
    ) -> Result<()> {
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn token_add_bank(
        ctx: Context<TokenAddBank>,
        token_index: TokenIndex,
        bank_num: u32,
    ) -> Result<()> {
        instructions::token_add_bank(ctx, token_index, bank_num)
    }

    pub fn token_deregister<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
    ) -> Result<()> {
        instructions::token_deregister(ctx)
    }

    pub fn token_update_index_and_rate(ctx: Context<TokenUpdateIndexAndRate>) -> Result<()> {
        instructions::token_update_index_and_rate(ctx)
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
        instructions::account_create(
            ctx,
            account_num,
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
            name,
        )
    }

    pub fn account_expand(
        ctx: Context<AccountExpand>,
        token_count: u8,
        serum3_count: u8,
        perp_count: u8,
        perp_oo_count: u8,
    ) -> Result<()> {
        instructions::account_expand(ctx, token_count, serum3_count, perp_count, perp_oo_count)
    }

    pub fn account_edit(
        ctx: Context<AccountEdit>,
        name_opt: Option<String>,
        delegate_opt: Option<Pubkey>,
    ) -> Result<()> {
        instructions::account_edit(ctx, name_opt, delegate_opt)
    }

    pub fn account_close(ctx: Context<AccountClose>) -> Result<()> {
        instructions::account_close(ctx)
    }

    // todo:
    // ckamm: generally, using an I80F48 arg will make it harder to call
    // because generic anchor clients won't know how to deal with it
    // and it's tricky to use in typescript generally
    // lets do an interface pass later
    pub fn stub_oracle_create(ctx: Context<StubOracleCreate>, price: I80F48) -> Result<()> {
        instructions::stub_oracle_create(ctx, price)
    }

    pub fn stub_oracle_close(ctx: Context<StubOracleClose>) -> Result<()> {
        instructions::stub_oracle_close(ctx)
    }

    pub fn stub_oracle_set(ctx: Context<StubOracleSet>, price: I80F48) -> Result<()> {
        instructions::stub_oracle_set(ctx, price)
    }

    pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64) -> Result<()> {
        instructions::token_deposit(ctx, amount)
    }

    pub fn token_deposit_into_existing(
        ctx: Context<TokenDepositIntoExisting>,
        amount: u64,
    ) -> Result<()> {
        instructions::token_deposit_into_existing(ctx, amount)
    }

    pub fn token_withdraw(
        ctx: Context<TokenWithdraw>,
        amount: u64,
        allow_borrow: bool,
    ) -> Result<()> {
        instructions::token_withdraw(ctx, amount, allow_borrow)
    }

    pub fn flash_loan_begin<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanBegin<'info>>,
        loan_amounts: Vec<u64>,
    ) -> Result<()> {
        instructions::flash_loan_begin(ctx, loan_amounts)
    }

    pub fn flash_loan_end<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanEnd<'info>>,
        flash_loan_type: FlashLoanType,
    ) -> Result<()> {
        instructions::flash_loan_end(ctx, flash_loan_type)
    }

    pub fn health_region_begin<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionBegin<'info>>,
    ) -> Result<()> {
        instructions::health_region_begin(ctx)
    }

    pub fn health_region_end<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionEnd<'info>>,
    ) -> Result<()> {
        instructions::health_region_end(ctx)
    }

    ///
    /// Serum
    ///

    // TODO deposit/withdraw msrm

    pub fn serum3_register_market(
        ctx: Context<Serum3RegisterMarket>,
        market_index: Serum3MarketIndex,
        name: String,
    ) -> Result<()> {
        instructions::serum3_register_market(ctx, market_index, name)
    }

    // note:
    // pub fn serum3_edit_market - doesn't exist since a mango serum3 market only contains the properties
    // registered base and quote token pairs, and serum3 external market its pointing to, and none of them
    // should be edited once set on creation

    pub fn serum3_deregister_market(ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
        instructions::serum3_deregister_market(ctx)
    }

    pub fn serum3_create_open_orders(ctx: Context<Serum3CreateOpenOrders>) -> Result<()> {
        instructions::serum3_create_open_orders(ctx)
    }

    pub fn serum3_close_open_orders(ctx: Context<Serum3CloseOpenOrders>) -> Result<()> {
        instructions::serum3_close_open_orders(ctx)
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
        )
    }

    pub fn serum3_cancel_order(
        ctx: Context<Serum3CancelOrder>,
        side: Serum3Side,
        order_id: u128,
    ) -> Result<()> {
        instructions::serum3_cancel_order(ctx, side, order_id)
    }

    pub fn serum3_cancel_all_orders(ctx: Context<Serum3CancelAllOrders>, limit: u8) -> Result<()> {
        instructions::serum3_cancel_all_orders(ctx, limit)
    }

    pub fn serum3_settle_funds(ctx: Context<Serum3SettleFunds>) -> Result<()> {
        instructions::serum3_settle_funds(ctx)
    }

    pub fn serum3_liq_force_cancel_orders(
        ctx: Context<Serum3LiqForceCancelOrders>,
        limit: u8,
    ) -> Result<()> {
        instructions::serum3_liq_force_cancel_orders(ctx, limit)
    }

    // DEPRECATED: use token_liq_with_token
    pub fn liq_token_with_token(
        ctx: Context<TokenLiqWithToken>,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        instructions::token_liq_with_token(
            ctx,
            asset_token_index,
            liab_token_index,
            max_liab_transfer,
        )
    }

    // DEPRECATED: use token_liq_bankruptcy
    pub fn liq_token_bankruptcy(
        ctx: Context<TokenLiqBankruptcy>,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        instructions::token_liq_bankruptcy(ctx, max_liab_transfer)
    }

    pub fn token_liq_with_token(
        ctx: Context<TokenLiqWithToken>,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        instructions::token_liq_with_token(
            ctx,
            asset_token_index,
            liab_token_index,
            max_liab_transfer,
        )
    }

    pub fn token_liq_bankruptcy(
        ctx: Context<TokenLiqBankruptcy>,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        instructions::token_liq_bankruptcy(ctx, max_liab_transfer)
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
        maint_asset_weight: f32,
        init_asset_weight: f32,
        maint_liab_weight: f32,
        init_liab_weight: f32,
        liquidation_fee: f32,
        maker_fee: f32,
        taker_fee: f32,
        min_funding: f32,
        max_funding: f32,
        impact_quantity: i64,
        group_insurance_fund: bool,
        trusted_market: bool,
        fee_penalty: f32,
        settle_fee_flat: f32,
        settle_fee_amount_threshold: f32,
        settle_fee_fraction_low_health: f32,
        settle_token_index: TokenIndex,
    ) -> Result<()> {
        instructions::perp_create_market(
            ctx,
            settle_token_index,
            perp_market_index,
            name,
            oracle_config,
            base_decimals,
            quote_lot_size,
            base_lot_size,
            maint_asset_weight,
            init_asset_weight,
            maint_liab_weight,
            init_liab_weight,
            liquidation_fee,
            maker_fee,
            taker_fee,
            min_funding,
            max_funding,
            impact_quantity,
            group_insurance_fund,
            trusted_market,
            fee_penalty,
            settle_fee_flat,
            settle_fee_amount_threshold,
            settle_fee_fraction_low_health,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perp_edit_market(
        ctx: Context<PerpEditMarket>,
        oracle_opt: Option<Pubkey>,
        oracle_config_opt: Option<OracleConfigParams>,
        base_decimals_opt: Option<u8>,
        maint_asset_weight_opt: Option<f32>,
        init_asset_weight_opt: Option<f32>,
        maint_liab_weight_opt: Option<f32>,
        init_liab_weight_opt: Option<f32>,
        liquidation_fee_opt: Option<f32>,
        maker_fee_opt: Option<f32>,
        taker_fee_opt: Option<f32>,
        min_funding_opt: Option<f32>,
        max_funding_opt: Option<f32>,
        impact_quantity_opt: Option<i64>,
        group_insurance_fund_opt: Option<bool>,
        trusted_market_opt: Option<bool>,
        fee_penalty_opt: Option<f32>,
        settle_fee_flat_opt: Option<f32>,
        settle_fee_amount_threshold_opt: Option<f32>,
        settle_fee_fraction_low_health_opt: Option<f32>,
    ) -> Result<()> {
        instructions::perp_edit_market(
            ctx,
            oracle_opt,
            oracle_config_opt,
            base_decimals_opt,
            maint_asset_weight_opt,
            init_asset_weight_opt,
            maint_liab_weight_opt,
            init_liab_weight_opt,
            liquidation_fee_opt,
            maker_fee_opt,
            taker_fee_opt,
            min_funding_opt,
            max_funding_opt,
            impact_quantity_opt,
            group_insurance_fund_opt,
            trusted_market_opt,
            fee_penalty_opt,
            settle_fee_flat_opt,
            settle_fee_amount_threshold_opt,
            settle_fee_fraction_low_health_opt,
        )
    }

    pub fn perp_close_market(ctx: Context<PerpCloseMarket>) -> Result<()> {
        instructions::perp_close_market(ctx)
    }

    pub fn perp_deactivate_position(ctx: Context<PerpDeactivatePosition>) -> Result<()> {
        instructions::perp_deactivate_position(ctx)
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
        // Timestamps in the future are reduced to now + 255s.
        expiry_timestamp: u64,

        // Maximum number of orders from the book to fill.
        //
        // Use this to limit compute used during order matching.
        // When the limit is reached, processing stops and the instruction succeeds.
        limit: u8,
    ) -> Result<()> {
        require_gte!(price_lots, 0);

        use crate::state::{Order, OrderParams};
        let time_in_force = match Order::tif_from_expiry(expiry_timestamp) {
            Some(t) => t,
            None => {
                msg!("Order is already expired");
                return Ok(());
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
        instructions::perp_place_order(ctx, order, limit)
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
        // Timestamps in the future are reduced to now + 255s.
        expiry_timestamp: u64,

        // Maximum number of orders from the book to fill.
        //
        // Use this to limit compute used during order matching.
        // When the limit is reached, processing stops and the instruction succeeds.
        limit: u8,
    ) -> Result<()> {
        require_gte!(peg_limit, -1);

        use crate::state::{Order, OrderParams};
        let time_in_force = match Order::tif_from_expiry(expiry_timestamp) {
            Some(t) => t,
            None => {
                msg!("Order is already expired");
                return Ok(());
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
            },
        };
        instructions::perp_place_order(ctx, order, limit)
    }

    pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: u128) -> Result<()> {
        instructions::perp_cancel_order(ctx, order_id)
    }

    pub fn perp_cancel_order_by_client_order_id(
        ctx: Context<PerpCancelOrderByClientOrderId>,
        client_order_id: u64,
    ) -> Result<()> {
        instructions::perp_cancel_order_by_client_order_id(ctx, client_order_id)
    }

    pub fn perp_cancel_all_orders(ctx: Context<PerpCancelAllOrders>, limit: u8) -> Result<()> {
        instructions::perp_cancel_all_orders(ctx, limit)
    }

    pub fn perp_cancel_all_orders_by_side(
        ctx: Context<PerpCancelAllOrdersBySide>,
        side_option: Option<Side>,
        limit: u8,
    ) -> Result<()> {
        instructions::perp_cancel_all_orders_by_side(ctx, side_option, limit)
    }

    pub fn perp_consume_events(ctx: Context<PerpConsumeEvents>, limit: usize) -> Result<()> {
        instructions::perp_consume_events(ctx, limit)
    }

    pub fn perp_update_funding(ctx: Context<PerpUpdateFunding>) -> Result<()> {
        instructions::perp_update_funding(ctx)
    }

    pub fn perp_settle_pnl(ctx: Context<PerpSettlePnl>) -> Result<()> {
        instructions::perp_settle_pnl(ctx)
    }

    pub fn perp_settle_fees(ctx: Context<PerpSettleFees>, max_settle_amount: u64) -> Result<()> {
        instructions::perp_settle_fees(ctx, max_settle_amount)
    }

    pub fn perp_liq_base_position(
        ctx: Context<PerpLiqBasePosition>,
        max_base_transfer: i64,
    ) -> Result<()> {
        instructions::perp_liq_base_position(ctx, max_base_transfer)
    }

    pub fn perp_liq_force_cancel_orders(
        ctx: Context<PerpLiqForceCancelOrders>,
        limit: u8,
    ) -> Result<()> {
        instructions::perp_liq_force_cancel_orders(ctx, limit)
    }

    pub fn perp_liq_bankruptcy(
        ctx: Context<PerpLiqBankruptcy>,
        max_liab_transfer: u64,
    ) -> Result<()> {
        instructions::perp_liq_bankruptcy(ctx, max_liab_transfer)
    }

    pub fn alt_set(ctx: Context<AltSet>, index: u8) -> Result<()> {
        instructions::alt_set(ctx, index)
    }

    pub fn alt_extend(
        ctx: Context<AltExtend>,
        index: u8,
        new_addresses: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::alt_extend(ctx, index, new_addresses)
    }

    pub fn compute_account_data(ctx: Context<ComputeAccountData>) -> Result<()> {
        instructions::compute_account_data(ctx)
    }

    ///
    /// benchmark
    ///

    pub fn benchmark(ctx: Context<Benchmark>) -> Result<()> {
        instructions::benchmark(ctx)
    }
}

#[derive(Clone)]
pub struct Mango;

impl anchor_lang::Id for Mango {
    fn id() -> Pubkey {
        ID
    }
}
