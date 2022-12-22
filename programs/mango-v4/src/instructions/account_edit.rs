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
) -> Result<()> {
    require!(
        name_opt.is_some() || delegate_opt.is_some(),
        MangoError::SomeError
    );

    let mut account = ctx.accounts.account.load_full_mut()?;

    // note: unchanged fields are inline, and match exact definition in create_account
    // please maintain, and don't remove, makes it easy to reason about which support modification by owner

    if let Some(name) = name_opt {
        account.fixed.name = fill_from_str(&name)?;
    }

    // unchanged -
    // owner
    // account_num
    // bump

    if let Some(delegate) = delegate_opt {
        account.fixed.delegate = delegate;
    }

    // unchanged -
    // tokens
    // serum3
    // perps
    // being_liquidated
    // is_bankrupt

    Ok(())
}
