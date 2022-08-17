use anchor_lang::prelude::*;

use crate::address_lookup_table;
use crate::state::*;

#[derive(Accounts)]
pub struct AltExtend<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    /// CHECK
    pub address_lookup_table: UncheckedAccount<'info>, // TODO: wrapper?
    /// CHECK
    pub address_lookup_table_program: UncheckedAccount<'info>, // TODO: force address?

    #[account(mut)]
    pub payer: Signer<'info>,
}

pub fn alt_extend(ctx: Context<AltExtend>) -> Result<()> {
    let mut mint_info = ctx.accounts.mint_info.load_mut()?;

    let alt_previous_size =
        address_lookup_table::addresses(&ctx.accounts.address_lookup_table.try_borrow_data()?)
            .len();

    address_lookup_table::extend(
        ctx.accounts.address_lookup_table.to_account_info(),
        // TODO: is using the admin as ALT authority a good idea?
        ctx.accounts.admin.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        &[],
        vec![mint_info.first_bank(), mint_info.oracle],
    )?;

    mint_info.address_lookup_table_bank_index = alt_previous_size as u8;
    mint_info.address_lookup_table_oracle_index = alt_previous_size as u8 + 1;

    Ok(())
}
