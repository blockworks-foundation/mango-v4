use anchor_lang::prelude::*;

use crate::error::*;

#[derive(Accounts)]
pub struct Initialize {}

pub fn handler(ctx: Context<Initialize>) -> ProgramResult {
    require!(1 == 1, ErrorCode::SomeError);
    Ok(())
}
