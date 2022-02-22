#[macro_use]
extern crate static_assertions;

use anchor_lang::prelude::*;

mod error;
mod instructions;
mod state;

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
}

#[derive(Clone)]
pub struct Mango;

impl anchor_lang::Id for Mango {
    fn id() -> Pubkey {
        ID
    }
}
