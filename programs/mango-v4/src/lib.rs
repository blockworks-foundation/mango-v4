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
pub mod types;

use state::{OrderType, PerpMarketIndex, Serum3MarketIndex, Side, TokenIndex};

declare_id!("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");


/*
dd TODOs
1. oracle peg order
    we really need to figure out a passive liquidity solution
2. make liquidation possible with passive liquidity (?)
3. remove FIFO component in liquidation
4. add delay to taker orders
5. rewards for perps (?)
6. add origination fees for serum limit orders
7. socialize loss on perps should only hit the opposite side of this contract
8. deposit should also update the bank first
9. interest rate should be based on oracle
10. decentralized incentives to run the keeper
11. open interest limits
12. net deposit limits
13. keep track globally everytime new OpenOrders is created so we can know when we have closed all of them
 */


#[program]
pub mod mango_v4 {

    use std::str::FromStr;

    use solana_program::{log::sol_log_compute_units, program_memory::sol_memcmp};

    use super::*;

    pub fn create_group(ctx: Context<CreateGroup>) -> Result<()> {
        instructions::create_group(ctx)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_token(
        ctx: Context<RegisterToken>,
        token_index: TokenIndex,
        name: String,
        interest_rate_params: InterestRateParams,
        maint_asset_weight: f32,
        init_asset_weight: f32,
        maint_liab_weight: f32,
        init_liab_weight: f32,
        liquidation_fee: f32,
    ) -> Result<()> {
        instructions::register_token(
            ctx,
            token_index,
            name,
            interest_rate_params,
            maint_asset_weight,
            init_asset_weight,
            maint_liab_weight,
            init_liab_weight,
            liquidation_fee,
        )
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
        name: String,
    ) -> Result<()> {
        instructions::serum3_register_market(ctx, market_index, name)
    }

    pub fn serum3_create_open_orders(ctx: Context<Serum3CreateOpenOrders>) -> Result<()> {
        instructions::serum3_create_open_orders(ctx)
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

    pub fn serum3_settle_funds(ctx: Context<Serum3SettleFunds>) -> Result<()> {
        instructions::serum3_settle_funds(ctx)
    }

    pub fn serum3_liq_force_cancel_orders(
        ctx: Context<Serum3LiqForceCancelOrders>,
        limit: u8,
    ) -> Result<()> {
        instructions::serum3_liq_force_cancel_orders(ctx, limit)
    }

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
        instructions::perp_create_market(
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

    pub fn perp_consume_events(ctx: Context<PerpConsumeEvents>, limit: usize) -> Result<()> {
        instructions::perp_consume_events(ctx, limit)
    }

    ///
    /// benchmark
    ///
    ///
    pub fn benchmark(_ctx: Context<Benchmark>) -> Result<()> {
        // 101000
        // 477
        sol_log_compute_units(); // 100422

        sol_log_compute_units(); // 100321 -> 101

        msg!("msg!"); // 100079+101 -> 203
        sol_log_compute_units(); // 100117

        let pk1 = Pubkey::default(); // 10
        sol_log_compute_units(); // 100006
        let pk2 = Pubkey::default(); // 10
        sol_log_compute_units(); // 99895

        let _ = pk1 == pk2; // 56
        sol_log_compute_units(); // 99739

        let _ = sol_memcmp(&pk1.to_bytes(), &pk2.to_bytes(), 32); // 64
        sol_log_compute_units(); // 99574

        let large_number = I80F48::from_str("777472127991.999999999999996").unwrap();
        let half = I80F48::MAX / 2;
        let max = I80F48::MAX;
        sol_log_compute_units(); // 92610

        let _ = checked_math!(half + half); // 0
        sol_log_compute_units(); // 92509

        let _ = checked_math!(max - max); // 0
        sol_log_compute_units(); // 92408

        let _ = checked_math!(large_number * large_number); // 77
        sol_log_compute_units(); // 92230

        // /
        let _ = checked_math!(I80F48::ZERO / max); // 839
        sol_log_compute_units(); // 91290

        let _ = checked_math!(half / max); // 3438
        sol_log_compute_units(); // 87751

        let _ = checked_math!(max / max); // 3457
        sol_log_compute_units(); // 84193

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

#[derive(Accounts)]
pub struct Benchmark {}
