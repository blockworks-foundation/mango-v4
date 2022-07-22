use crate::error::MangoError;
use anchor_lang::prelude::*;

use crate::state::*;
use crate::util::fill32_from_str;

#[derive(Accounts)]
pub struct AccountEdit<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut)]
    pub account: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
}

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

    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new(&ctx.accounts.account)?;
    let mut account: MangoAccountAccMut = mal.load_mut()?;
    require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
    require_keys_eq!(account.fixed.owner, ctx.accounts.owner.key());

    // note: unchanged fields are inline, and match exact definition in create_account
    // please maintain, and don't remove, makes it easy to reason about which support modification by owner

    if let Some(name) = name_opt {
        account.fixed.name = fill32_from_str(name)?;
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
