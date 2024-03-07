use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn token_create_position(
    ctx: Context<TokenCreateOrClosePosition>,
    allow_lending: bool,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    require!(
        group.is_ix_enabled(IxGate::TokenCreatePosition),
        MangoError::IxIsDisabled
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let bank = ctx.accounts.bank.load()?;

    // If there is one already, make it a no-op
    if let Ok(tp) = account.token_position(bank.token_index) {
        require_eq!(
            tp.allow_lending(),
            allow_lending,
            MangoError::TokenPositionWithDifferentSettingAlreadyExists
        );
        return Ok(());
    }

    let (tp, _, _) = account.ensure_token_position(bank.token_index)?;
    tp.disable_lending = if allow_lending { 0 } else { 1 };

    Ok(())
}
