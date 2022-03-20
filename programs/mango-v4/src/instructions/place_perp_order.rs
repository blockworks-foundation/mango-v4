use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PlacePerpOrder {}

pub fn place_perp_order(_ctx: Context<PlacePerpOrder>) -> Result<()> {
    Ok(())
}
