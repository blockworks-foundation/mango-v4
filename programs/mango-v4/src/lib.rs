use fixed::types::I80F48;

#[macro_use]
pub mod util;

extern crate static_assertions;

use anchor_lang::prelude::*;

use instructions::*;

pub mod accounts_zerocopy;
pub mod address_lookup_table;
pub mod error;
pub mod instructions;
pub mod logs;
mod serum3_cpi;
pub mod state;
pub mod types;

use state::{
    HealthType, OracleConfig, OrderType, PerpMarketIndex, Serum3MarketIndex, Side, TokenIndex,
};

declare_id!("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");

#[program]
pub mod mango_v4 {

    use crate::state::OracleConfig;

    use super::*;

    pub fn create_group(ctx: Context<CreateGroup>, group_num: u32, testing: u8) -> Result<()> {
        instructions::create_group(ctx, group_num, testing)
    }

    pub fn close_group(ctx: Context<CloseGroup>) -> Result<()> {
        instructions::close_group(ctx)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn token_register(
        ctx: Context<TokenRegister>,
        token_index: TokenIndex,
        bank_num: u64,
        name: String,
        oracle_config: OracleConfig,
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
            bank_num,
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

    #[allow(clippy::too_many_arguments)]
    pub fn token_add_bank(
        ctx: Context<TokenAddBank>,
        token_index: TokenIndex,
        bank_num: u64,
    ) -> Result<()> {
        instructions::token_add_bank(ctx, token_index, bank_num)
    }

    pub fn token_deregister<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
        token_index: TokenIndex,
    ) -> Result<()> {
        instructions::token_deregister(ctx, token_index)
    }

    pub fn update_index(ctx: Context<UpdateIndex>) -> Result<()> {
        instructions::update_index(ctx)
    }

    pub fn create_account(
        ctx: Context<CreateAccount>,
        account_num: u8,
        name: String,
    ) -> Result<()> {
        instructions::create_account(ctx, account_num, name)
    }

    // TODO set delegate

    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
        instructions::close_account(ctx)
    }

    // todo:
    // ckamm: generally, using an I80F48 arg will make it harder to call
    // because generic anchor clients won't know how to deal with it
    // and it's tricky to use in typescript generally
    // lets do an interface pass later
    pub fn create_stub_oracle(ctx: Context<CreateStubOracle>, price: I80F48) -> Result<()> {
        instructions::create_stub_oracle(ctx, price)
    }

    pub fn close_stub_oracle(ctx: Context<CloseStubOracle>) -> Result<()> {
        instructions::close_stub_oracle(ctx)
    }

    pub fn set_stub_oracle(ctx: Context<SetStubOracle>, price: I80F48) -> Result<()> {
        instructions::set_stub_oracle(ctx, price)
    }

    pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64) -> Result<()> {
        instructions::token_deposit(ctx, amount)
    }

    pub fn token_withdraw(
        ctx: Context<TokenWithdraw>,
        amount: u64,
        allow_borrow: bool,
    ) -> Result<()> {
        instructions::token_withdraw(ctx, amount, allow_borrow)
    }

    pub fn flash_loan<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoan<'info>>,
        withdraws: Vec<FlashLoanWithdraw>,
        cpi_datas: Vec<CpiData>,
    ) -> Result<()> {
        instructions::flash_loan(ctx, withdraws, cpi_datas)
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

    pub fn serum3_deregister_market(ctx: Context<Serum3DeregisterMarket>) -> Result<()> {
        instructions::serum3_deregister_market(ctx)
    }

    // TODO serum3_change_spot_market_params

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

    // TODO serum3_cancel_all_spot_orders

    pub fn liq_token_with_token(
        ctx: Context<LiqTokenWithToken>,
        asset_token_index: TokenIndex,
        liab_token_index: TokenIndex,
        max_liab_transfer: I80F48,
    ) -> Result<()> {
        instructions::liq_token_with_token(
            ctx,
            asset_token_index,
            liab_token_index,
            max_liab_transfer,
        )
    }

    ///
    /// Perps
    ///

    #[allow(clippy::too_many_arguments)]
    pub fn perp_create_market(
        ctx: Context<PerpCreateMarket>,
        perp_market_index: PerpMarketIndex,
        name: String,
        oracle_config: OracleConfig,
        base_token_index_opt: Option<TokenIndex>,
        base_token_decimals: u8,
        quote_token_index: TokenIndex,
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
    ) -> Result<()> {
        instructions::perp_create_market(
            ctx,
            perp_market_index,
            name,
            oracle_config,
            base_token_index_opt,
            base_token_decimals,
            quote_token_index,
            quote_lot_size,
            base_lot_size,
            maint_asset_weight,
            init_asset_weight,
            maint_liab_weight,
            init_liab_weight,
            liquidation_fee,
            maker_fee,
            taker_fee,
            max_funding,
            min_funding,
            impact_quantity,
        )
    }

    pub fn perp_close_market(ctx: Context<PerpCloseMarket>) -> Result<()> {
        instructions::perp_close_market(ctx)
    }

    // TODO perp_change_perp_market_params

    #[allow(clippy::too_many_arguments)]
    pub fn perp_place_order(
        ctx: Context<PerpPlaceOrder>,
        side: Side,
        price_lots: i64,
        max_base_lots: i64,
        max_quote_lots: i64,
        client_order_id: u64,
        order_type: OrderType,
        expiry_timestamp: u64,
        limit: u8,
    ) -> Result<()> {
        instructions::perp_place_order(
            ctx,
            side,
            price_lots,
            max_base_lots,
            max_quote_lots,
            client_order_id,
            order_type,
            expiry_timestamp,
            limit,
        )
    }

    pub fn perp_cancel_order(ctx: Context<PerpCancelOrder>, order_id: i128) -> Result<()> {
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

    // TODO

    // perp_force_cancel_order

    // liquidate_token_and_perp
    // liquidate_perp_and_perp

    // settle_* - settle_funds, settle_pnl, settle_fees

    // resolve_banktruptcy

    pub fn compute_health(ctx: Context<ComputeHealth>, health_type: HealthType) -> Result<I80F48> {
        instructions::compute_health(ctx, health_type)
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
