use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct AccountExpand<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner
    )]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn account_expand(ctx: Context<AccountExpand>) -> Result<()> {
    let account_size = {
        let account = ctx.accounts.account.load()?;
        account.size()
    };

    require_eq!(account_size, AccountSize::Small);

    let new_space = MangoAccount::space(AccountSize::Large);
    let new_rent_minimum = Rent::get()?.minimum_balance(new_space);

    let realloc_account = ctx.accounts.account.as_ref();
    let old_space = realloc_account.data_len();

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
        new_rent_minimum
            .checked_sub(realloc_account.lamports())
            .unwrap(),
    )?;

    // realloc
    realloc_account.realloc(new_space, true)?;

    // expand dynamic content, e.g. to grow token positions, we need to slide serum3orders further later, and so on....
    let mut account = ctx.accounts.account.load_mut()?;
    account.expand_dynamic_content(AccountSize::Large)?;

    Ok(())
}
