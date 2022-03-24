use fixed::types::I80F48;

#[macro_use]
pub mod util;

extern crate static_assertions;

use anchor_lang::prelude::*;

use instructions::*;

pub mod address_lookup_table;
pub mod error;
pub mod instructions;
mod serum3_cpi;
pub mod state;

use state::{OrderType, PerpMarketIndex, Serum3MarketIndex, TokenIndex};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod mango_v4 {

    use super::*;

    pub fn create_group(ctx: Context<CreateGroup>) -> Result<()> {
        instructions::create_group(ctx)
    }

    pub fn register_token(
        ctx: Context<RegisterToken>,
        token_index: TokenIndex,
        maint_asset_weight: f32,
        init_asset_weight: f32,
        maint_liab_weight: f32,
        init_liab_weight: f32,
    ) -> Result<()> {
        instructions::register_token(
            ctx,
            token_index,
            maint_asset_weight,
            init_asset_weight,
            maint_liab_weight,
            init_liab_weight,
        )
    }

    pub fn create_account(ctx: Context<CreateAccount>, account_num: u8) -> Result<()> {
        instructions::create_account(ctx, account_num)
    }

    // todo:
    // ckamm: generally, using an I80F48 arg will make it harder to call
    // because generic anchor clients won't know how to deal with it
    // and it's tricky to use in typescript generally
    // lets do an interface pass later
    pub fn create_stub_oracle(ctx: Context<CreateStubOracle>, price: I80F48) -> Result<()> {
        instructions::create_stub_oracle(ctx, price)
    }

    pub fn set_stub_oracle(ctx: Context<SetStubOracle>, price: I80F48) -> Result<()> {
        instructions::set_stub_oracle(ctx, price)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
        instructions::withdraw(ctx, amount, allow_borrow)
    }

    pub fn margin_trade<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, MarginTrade<'info>>,
        banks_len: usize,
        cpi_data: Vec<u8>,
    ) -> Result<()> {
        instructions::margin_trade(ctx, banks_len, cpi_data)
    }

    ///
    /// Serum
    ///

    pub fn serum3_register_market(
        ctx: Context<Serum3RegisterMarket>,
        market_index: Serum3MarketIndex,
        base_token_index: TokenIndex,
        quote_token_index: TokenIndex,
    ) -> Result<()> {
        instructions::serum3_register_market(ctx, market_index, base_token_index, quote_token_index)
    }

    pub fn serum3_create_open_orders(ctx: Context<Serum3CreateOpenOrders>) -> Result<()> {
        instructions::serum3_create_open_orders(ctx)
    }

    pub fn serum3_place_order(
        ctx: Context<Serum3PlaceOrder>,
        order: instructions::NewOrderInstructionData,
    ) -> Result<()> {
        instructions::serum3_place_order(ctx, order)
    }

    pub fn serum3_cancel_order(
        ctx: Context<Serum3CancelOrder>,
        order: instructions::CancelOrderInstructionData,
    ) -> Result<()> {
        instructions::serum3_cancel_order(ctx, order)
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

    ///
    /// Perps
    ///

    #[allow(clippy::too_many_arguments)]
    pub fn create_perp_market(
        ctx: Context<CreatePerpMarket>,
        perp_market_index: PerpMarketIndex,
        base_token_index_opt: Option<TokenIndex>,
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
    ) -> Result<()> {
        instructions::create_perp_market(
            ctx,
            perp_market_index,
            base_token_index_opt,
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn place_perp_order(
        ctx: Context<PlacePerpOrder>,
        price: i64,
        max_base_quantity: i64,
        max_quote_quantity: i64,
        client_order_id: u64,
        order_type: OrderType,
        expiry_timestamp: u64,
        limit: u8,
    ) -> Result<()> {
        instructions::place_perp_order(
            ctx,
            price,
            max_base_quantity,
            max_quote_quantity,
            client_order_id,
            order_type,
            expiry_timestamp,
            limit,
        )
    }
}

#[derive(Clone)]
pub struct Mango;

impl anchor_lang::Id for Mango {
    fn id() -> Pubkey {
        ID
    }
}
