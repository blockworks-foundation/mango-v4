use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::state::*;

pub fn account_expand(
    ctx: Context<AccountExpand>,
    token_count: u8,
    serum3_count: u8,
    perp_count: u8,
    perp_oo_count: u8,
) -> Result<()> {
    let new_space = MangoAccount::space(token_count, serum3_count, perp_count, perp_oo_count)?;
    let new_rent_minimum = Rent::get()?.minimum_balance(new_space);

    let realloc_account = ctx.accounts.account.as_ref();
    let old_space = realloc_account.data_len();
    let old_lamports = realloc_account.lamports();

    require_gt!(new_space, old_space);

    // transfer required additional rent
    anchor_lang::system_program::transfer(
        anchor_lang::context::CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: realloc_account.clone(),
            },
        ),
        new_rent_minimum - old_lamports,
    )?;

    // realloc: it's safe to not re-zero-init since we never shrink accounts
    realloc_account.realloc(new_space, false)?;

    // expand dynamic content, e.g. to grow token positions, we need to slide serum3orders further later, and so on....
    let mut account = ctx.accounts.account.load_full_mut()?;
    account.expand_dynamic_content(token_count, serum3_count, perp_count, perp_oo_count)?;

    Ok(())
}
