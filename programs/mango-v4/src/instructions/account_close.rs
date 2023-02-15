use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn account_close(ctx: Context<AccountClose>, force_close: bool) -> Result<()> {
    let account = ctx.accounts.account.load_full_mut()?;

    if !ctx.accounts.group.load()?.is_testing() {
        require!(!force_close, MangoError::SomeError);
    }

    if !force_close {
        require!(!account.fixed.being_liquidated(), MangoError::SomeError);
        for ele in account.all_token_positions() {
            require_eq!(ele.is_active(), false);
        }
        for ele in account.all_serum3_orders() {
            require_eq!(ele.is_active(), false);
        }
        for ele in account.all_perp_positions() {
            require_eq!(ele.is_active(), false);
        }
    }

    Ok(())
}
