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

    {
        let mut mal: MangoAccountLoader<MangoAccount> =
            MangoAccountLoader::new(&ctx.accounts.account)?;
        let account: MangoAccountAccMut = mal.load_mut()?;
        require_keys_eq!(account.fixed.group, ctx.accounts.group.key());
        require_keys_eq!(account.fixed.owner, ctx.accounts.owner.key());

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
    }

    close(
        ctx.accounts.account.to_account_info(),
        ctx.accounts.sol_destination.to_account_info(),
    )?;

    Ok(())
}

// https://github.com/coral-xyz/anchor/blob/master/lang/src/common.rs#L8
pub fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;
    // Mark the account discriminator as closed.
    let mut data = info.try_borrow_mut_data()?;
    let dst: &mut [u8] = &mut data;
    dst[0..8].copy_from_slice(&[255, 255, 255, 255, 255, 255, 255, 255]);

    Ok(())
}
