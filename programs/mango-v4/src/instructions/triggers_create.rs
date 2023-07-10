use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn triggers_create(ctx: Context<TriggersCreate>) -> Result<()> {
    {
        let account = ctx.accounts.account.load()?;
        // account constraint #1
        require!(
            account.is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );
    }

    let mut triggers = ctx.accounts.triggers.load_init()?;
    *triggers = Triggers {
        group: ctx.accounts.group.key(),
        account: ctx.accounts.account.key(),
        next_trigger_id: 1,
        reserved: [0; 960],
    };

    Ok(())
}
