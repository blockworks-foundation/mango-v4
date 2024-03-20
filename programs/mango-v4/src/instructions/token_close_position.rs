use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;

pub fn token_close_position(ctx: Context<TokenCreateOrClosePosition>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    require!(
        group.is_ix_enabled(IxGate::TokenClosePosition),
        MangoError::IxIsDisabled
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let bank = ctx.accounts.bank.load()?;

    let (tp, raw_index) = account.token_position_and_raw_index(bank.token_index)?;
    require!(!tp.is_in_use(), MangoError::TokenPositionIsInUse);
    require_eq!(
        tp.native(&bank),
        I80F48::ZERO,
        MangoError::TokenPositionBalanceNotZero
    );

    account.deactivate_token_position_and_log(raw_index, ctx.accounts.account.key());

    Ok(())
}
