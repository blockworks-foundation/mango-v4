use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::state::*;

// Freezing an account, prevents all instructions involving account (also settling and liquidation), except
// perp consume events and force cancellation of orders
pub fn account_toggle_freeze(ctx: Context<AccountToggleFreeze>, freeze: bool) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    if freeze {
        let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        account.fixed.frozen_until = now_ts + 7 * 24 * 60 * 60;
    } else {
        account.fixed.frozen_until = 0;
    }

    Ok(())
}
