use anchor_lang::prelude::*;

use crate::error::*;

#[derive(Accounts)]
pub struct Initialize {}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    require!(1 == 1, MangoError::SomeError);
    Ok(())
}
