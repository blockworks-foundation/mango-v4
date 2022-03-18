use fixed::types::I80F48;

#[macro_use]
pub mod util;

extern crate static_assertions;

use anchor_lang::prelude::*;

use instructions::*;

pub mod address_lookup_table;
pub mod error;
pub mod instructions;
pub mod state;

use state::{Serum3MarketIndex, TokenIndex};

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
}

#[derive(Clone)]
pub struct Mango;

impl anchor_lang::Id for Mango {
    fn id() -> Pubkey {
        ID
    }
}
