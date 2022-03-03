use crate::error::MangoError;
use crate::state::{compute_health, MangoAccount, MangoGroup};
use crate::util::to_account_meta;
use crate::{group_seeds, Mango};
use anchor_lang::prelude::*;
use solana_program::instruction::Instruction;

#[derive(Accounts)]
pub struct MarginTrade<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,
}

/// reference https://github.com/blockworks-foundation/mango-v3/blob/mc/flash_loan/program/src/processor.rs#L5323
pub fn margin_trade(ctx: Context<MarginTrade>, cpi_data: Vec<u8>) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut account = ctx.accounts.account.load_mut()?;
    let active_len = account.indexed_positions.iter_active().count();
    let banks = &ctx.remaining_accounts[0..active_len];
    let oracles = &ctx.remaining_accounts[active_len..active_len * 2];
    let cpi_ais = &ctx.remaining_accounts[active_len * 2..];

    // since we are using group signer seeds to invoke cpi,
    // assert that none of the cpi accounts is the mango program to prevent that invoker doesn't
    // abuse this ix to do unwanted changes
    for cpi_ai in cpi_ais {
        require!(
            *ctx.remaining_accounts[active_len].key != Mango::id(),
            MangoError::InvalidMarginTradeTargetCpiProgram
        );
    }

    // compute pre cpi health
    let pre_cpi_health = compute_health(&mut account, &banks, &oracles).unwrap();
    require!(pre_cpi_health > 0, MangoError::HealthMustBePositive);

    // prepare and invoke cpi
    let cpi_ix = Instruction {
        program_id: *ctx.remaining_accounts[active_len].key,
        data: cpi_data,
        accounts: cpi_ais
            .iter()
            .skip(1)
            .map(|cpi_ai| to_account_meta(cpi_ai))
            .collect(),
    };
    let group_seeds = group_seeds!(group);
    solana_program::program::invoke_signed(&cpi_ix, &cpi_ais, &[group_seeds])?;

    // compute post cpi health
    let post_cpi_health = compute_health(&mut account, &banks, &oracles).unwrap();
    require!(post_cpi_health > 0, MangoError::HealthMustBePositive);

    Ok(())
}
