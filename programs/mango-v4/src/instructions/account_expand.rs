use anchor_lang::prelude::*;
use anchor_spl::token;

use crate::state::*;

#[derive(Accounts)]
pub struct AccountExpand<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        has_one = group,
        has_one = owner
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub account2: UncheckedAccount<'info>,
}

// https://github.com/coral-xyz/anchor/blob/master/lang/syn/src/codegen/accounts/constraints.rs#L328
pub fn account_expand(ctx: Context<AccountExpand>) -> Result<()> {
    // expand to these lengths
    let token_count = 5;
    let serum3_count = 6;
    let perp_count = 4;

    let new_space = MangoAccount2::space(token_count, serum3_count, perp_count);
    let new_rent_minimum = Rent::get()?.minimum_balance(new_space);

    let old_space = ctx.accounts.account2.data_len();

    require_gt!(new_space, old_space);

    // transfer required additional rent
    anchor_lang::system_program::transfer(
        anchor_lang::context::CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.account2.to_account_info(),
            },
        ),
        new_rent_minimum
            .checked_sub(ctx.accounts.account2.lamports())
            .unwrap(),
    )?;

    // realloc
    ctx.accounts.account2.realloc(new_space, true)?;

    // expand dynamic content, e.g. to grow token positions, we need to slide serum3orders further later, and so on....
    let mal: MangoAccountLoader<MangoAccount2Fixed, MangoAccount2DynamicHeader, MangoAccount2> =
        MangoAccountLoader::new(ctx.accounts.account2.to_account_info());
    let mut meta = mal.load_mut()?;
    meta.dynamic
        .expand_dynamic_content(token_count, serum3_count, perp_count)?;

    Ok(())
}
