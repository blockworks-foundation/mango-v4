#[macro_use]
extern crate static_assertions;

use anchor_lang::prelude::*;

mod state;
mod instructions;
mod error;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod mango_v4 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {
        instructions::initialiaze::handler(ctx)
    }

}
