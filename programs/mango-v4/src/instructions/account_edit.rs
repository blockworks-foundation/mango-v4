use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;
use crate::util::fill_from_str;

pub fn account_edit(
    ctx: Context<AccountEdit>,
    name_opt: Option<String>,
    // note: can also be used to unset by using the default pubkey here as a param
    delegate_opt: Option<Pubkey>,
    delegate_expiry_opt: Option<u64>,
) -> Result<()> {
    require!(
        name_opt.is_some() || delegate_opt.is_some(),
        MangoError::SomeError
    );

    let mut account = ctx.accounts.account.load_full_mut()?;

    if let Some(name) = name_opt {
        account.fixed.name = fill_from_str(&name)?;
    }

    if let Some(delegate) = delegate_opt {
        account.fixed.delegate = delegate;
    }

    if let Some(delegate_expiry) = delegate_expiry_opt {
        account.fixed.delegate_expiry = delegate_expiry;
    }

    Ok(())
}
