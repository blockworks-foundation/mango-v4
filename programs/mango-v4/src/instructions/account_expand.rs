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
    let new_size = MangoAccount::space(
        token_count,
        serum3_count,
        perp_count,
        perp_oo_count,
        token_conditional_swap_count,
    );
    let new_rent_minimum = Rent::get()?.minimum_balance(new_size);

    let account_ai = ctx.accounts.account.as_ref();
    let current_size = account_ai.data_len();
    let current_lamports = account_ai.lamports();

    // Either get more lamports for rent, or transfer out the surplus
    if current_lamports < new_rent_minimum {
        anchor_lang::system_program::transfer(
            anchor_lang::context::CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: account_ai.clone(),
                },
            ),
            new_rent_minimum - current_lamports,
        )?;
    } else if current_lamports > new_rent_minimum {
        // Transfer out excess lamports, but can't use a data-having account in spl::Tranfer::from,
        // so adjust lamports manually.
        let excess = current_lamports - new_rent_minimum;
        let mut account_lamports = account_ai.try_borrow_mut_lamports()?;
        **account_lamports -= excess;
        let mut payer_lamports = ctx.accounts.payer.as_ref().try_borrow_mut_lamports()?;
        **payer_lamports += excess;
    }

    // This instruction has <= 1 calls to AccountInfo::realloc(), meaning that the
    // new data when expanding the account will be zero initialized already: it's not
    // necessary to zero init it again.
    let no_zero_init = false;

    if new_size > current_size {
        account_ai.realloc(new_size, no_zero_init)?;
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

    if new_size < current_size {
        account_ai.realloc(new_size, no_zero_init)?;
    }

    Ok(())
}
