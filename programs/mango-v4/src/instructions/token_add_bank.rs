use crate::accounts_ix::*;
use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn token_add_bank(
    ctx: Context<TokenAddBank>,
    token_index: TokenIndex,
    bank_num: u32,
) -> Result<()> {
    let existing_bank = ctx.accounts.existing_bank.load()?;
    let mut bank = ctx.accounts.bank.load_init()?;
    let bump = *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?;
    *bank = Bank::from_existing_bank(&existing_bank, ctx.accounts.vault.key(), bank_num, bump);

    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    let free_slot = mint_info
        .banks
        .iter()
        .position(|bank| bank == &Pubkey::default())
        .unwrap();
    require_eq!(bank_num as usize, free_slot);
    mint_info.banks[free_slot] = ctx.accounts.bank.key();
    mint_info.vaults[free_slot] = ctx.accounts.vault.key();

    Ok(())
}
