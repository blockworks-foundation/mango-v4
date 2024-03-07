use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;

pub fn sequence_check(ctx: Context<SequenceCheck>, expected_sequence_number: u64) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;

    require!(
        expected_sequence_number == account.fixed.sequence_number,
        MangoError::InvalidSequenceNumber
    );

    account.fixed.sequence_number = account.fixed.sequence_number.wrapping_add(1);
    Ok(())
}
