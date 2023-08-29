use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::state::*;

pub fn account_force_shrink(ctx: Context<AccountForceShrink>) -> Result<()> {
    let account = ctx.accounts.account.load_full()?;
    let current_header = account.header.clone();

    // If we could resize to the current size from 0 or the health accounts are fine, stop.
    let zero_header = MangoAccountDynamicHeader::zero();
    if let Ok(_) = current_header.check_resize_from(&zero_header) {
        return Ok(());
    }
    if current_header.expected_health_accounts() <= MangoAccountDynamicHeader::max_health_accounts()
    {
        return Ok(());
    }

    let new_header = MangoAccountDynamicHeader {
        token_count: current_header.token_count.min(
            account
                .active_token_positions()
                .count()
                .max(8)
                .try_into()
                .unwrap(),
        ),
        perp_count: current_header.perp_count.min(
            account
                .active_perp_positions()
                .count()
                .max(4)
                .try_into()
                .unwrap(),
        ),
        serum3_count: current_header.serum3_count.min(
            account
                .active_serum3_orders()
                .count()
                .max(4)
                .try_into()
                .unwrap(),
        ),
        ..current_header
    };
    drop(account);

    let new_space = new_header.account_size();
    let new_rent_minimum = Rent::get()?.minimum_balance(new_space);

    let realloc_account = ctx.accounts.account.as_ref();
    let old_space = realloc_account.data_len();
    let old_lamports = realloc_account.lamports();

    require_gte!(old_lamports, new_rent_minimum);

    // This instruction has <= 1 calls to AccountInfo::realloc(), meaning that the
    // new data when expanding the account will be zero initialized already: it's not
    // necessary to zero init it again.
    let no_zero_init = false;

    // resize the dynamic content on the account
    {
        let mut account = ctx.accounts.account.load_full_mut()?;
        account.resize_dynamic_content(
            new_header.token_count,
            new_header.serum3_count,
            new_header.perp_count,
            new_header.perp_oo_count,
            new_header.token_conditional_swap_count,
        )?;
    }

    if new_space < old_space {
        realloc_account.realloc(new_space, no_zero_init)?;
    }

    Ok(())
}
