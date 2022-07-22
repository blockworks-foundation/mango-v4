use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
pub struct AccountExpand<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut)]
    pub account: UncheckedAccount<'info>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn account_expand(ctx: Context<AccountExpand>) -> Result<()> {
    let account_size = {
        let mal: MangoAccountLoader<MangoAccount> = MangoAccountLoader::new(&ctx.accounts.account)?;
        let account = mal.load()?;
        account.size()
    };

    require_eq!(account_size, AccountSize::Small);

    let new_space = MangoAccount::space(AccountSize::Large.try_into().unwrap());
    let new_rent_minimum = Rent::get()?.minimum_balance(new_space);

    let old_space = ctx.accounts.account.data_len();

    require_gt!(new_space, old_space);

    // transfer required additional rent
    anchor_lang::system_program::transfer(
        anchor_lang::context::CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.account.to_account_info(),
            },
        ),
        new_rent_minimum
            .checked_sub(ctx.accounts.account.lamports())
            .unwrap(),
    )?;

    // realloc
    ctx.accounts.account.realloc(new_space, true)?;

    // expand dynamic content, e.g. to grow token positions, we need to slide serum3orders further later, and so on....
    let mut mal: MangoAccountLoader<MangoAccount> = MangoAccountLoader::new(&ctx.accounts.account)?;
    let mut account = mal.load_mut()?;
    require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
    require_keys_eq!(account.fixed.owner, ctx.accounts.owner.key());
    account.expand_dynamic_content(AccountSize::Large.try_into().unwrap())?;

    Ok(())
}
