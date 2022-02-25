#[macro_use]
extern crate static_assertions;

use anchor_lang::prelude::*;

mod error;
mod instructions;
pub mod state;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod mango_v4 {
    use super::*;

    pub fn create_group(ctx: Context<CreateGroup>) -> Result<()> {
        instructions::create_group(ctx)
    }

    pub fn register_token(ctx: Context<RegisterToken>, decimals: u8) -> Result<()> {
        instructions::register_token(ctx, decimals)
    }

    pub fn create_account(ctx: Context<CreateAccount>, account_num: u8) -> Result<()> {
        instructions::create_account(ctx, account_num)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
        instructions::withdraw(ctx, amount, allow_borrow)
    }
}

#[derive(Clone)]
pub struct Mango;

impl anchor_lang::Id for Mango {
    fn id() -> Pubkey {
        ID
    }
}
