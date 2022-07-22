use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct AccountClose<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(mut)]
    pub account: UncheckedAccount<'info>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn account_close(ctx: Context<AccountClose>) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new_init(&ctx.accounts.account)?;
    let account: MangoAccountAccMut = mal.load_mut()?;
    require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
    require_keys_eq!(account.fixed.owner, ctx.accounts.owner.key());

    // TODO: close account manually

    // don't perform checks if group is just testing
    if group.testing == 0 {
        require!(!account.fixed.being_liquidated(), MangoError::SomeError);
        require!(!account.fixed.is_bankrupt(), MangoError::SomeError);
        require_eq!(account.fixed.delegate, Pubkey::default());
        for ele in account.token_iter() {
            require_eq!(ele.is_active(), false);
        }
        for ele in account.serum3_iter() {
            require_eq!(ele.is_active(), false);
        }
        for ele in account.perp_iter() {
            require_eq!(ele.is_active(), false);
        }
    }

    Ok(())
}
