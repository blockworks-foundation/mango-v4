use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::state::*;

pub fn account_expand(
    ctx: Context<AccountExpand>,
    token_count: u8,
    serum3_count: u8,
    perp_count: u8,
    perp_oo_count: u8,
    token_conditional_swap_count: u8,
) -> Result<()> {
    let new_space = MangoAccount::space(
        token_count,
        serum3_count,
        perp_count,
        perp_oo_count,
        token_conditional_swap_count,
    );
    let new_rent_minimum = Rent::get()?.minimum_balance(new_space);

    let realloc_account = ctx.accounts.account.as_ref();
    let old_space = realloc_account.data_len();
    let old_lamports = realloc_account.lamports();

    // Either get more lamports for rent, or transfer out the surplus
    if old_lamports < new_rent_minimum {
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
    } else if old_lamports > new_rent_minimum {
        // Transfer out excess lamports, but can't use a data-having account in spl::Tranfer::from,
        // so adjust lamports manually.
        let excess = old_lamports - new_rent_minimum;
        let mut account_lamports = realloc_account.try_borrow_mut_lamports()?;
        **account_lamports -= excess;
        let mut payer_lamports = ctx.accounts.payer.as_ref().try_borrow_mut_lamports()?;
        **payer_lamports += excess;
    }

    // This instruction has <= 1 calls to AccountInfo::realloc(), meaning that the
    // new data when expanding the account will be zero initialized already: it's not
    // necessary to zero init it again.
    let no_zero_init = false;

    if new_space > old_space {
        realloc_account.realloc(new_space, no_zero_init)?;
    }

    // resize the dynamic content on the account
    {
        let mut account = ctx.accounts.account.load_full_mut()?;
        account.resize_dynamic_content(
            token_count,
            serum3_count,
            perp_count,
            perp_oo_count,
            token_conditional_swap_count,
        )?;
    }

    if new_space < old_space {
        realloc_account.realloc(new_space, no_zero_init)?;
    }

    Ok(())
}
