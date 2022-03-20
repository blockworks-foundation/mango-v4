use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PlacePerpOrder {}

pub fn place_perp_order(ctx: Context<PlacePerpOrder>) -> Result<()> {
    Ok(())
}
