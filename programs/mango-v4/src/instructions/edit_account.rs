use crate::error::MangoError;
use anchor_lang::prelude::*;

use crate::state::*;
use crate::util::fill32_from_str;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct EditAccount<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"MangoAccount".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        // Note: should never be the delegate
        has_one = owner,
        bump,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,
}

#[allow(unused_variables)]
pub fn edit_account(
    ctx: Context<EditAccount>,
    account_num: u8,
    name_opt: Option<String>,
    // note: can also be used to unset by using the default pubkey here as a param
    delegate_opt: Option<Pubkey>,
) -> Result<()> {
    require!(
        name_opt.is_some() || delegate_opt.is_some(),
        MangoError::SomeError
    );

    let mut account = ctx.accounts.account.load_mut()?;

    msg!("old account {:#?}", account);

    // note: unchanged fields are inline, and match exact definition in create_account
    // please maintain, and don't remove, makes it easy to reason about which support modification by owner

    if let Some(name) = name_opt {
        account.name = fill32_from_str(name)?;
    }

    // unchanged -
    // owner
    // account_num
    // bump

    if let Some(delegate) = delegate_opt {
        account.delegate = delegate;
    }

    // unchanged -
    // tokens
    // serum3
    // perps
    // being_liquidated
    // is_bankrupt

    msg!("new account {:#?}", account);

    Ok(())
}
