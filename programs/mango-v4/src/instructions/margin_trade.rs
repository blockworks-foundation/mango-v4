use crate::error::MangoError;
use crate::state::{compute_health, MangoAccount, MangoGroup, TokenBank};
use crate::{group_seeds, util, Mango};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use solana_program::instruction::Instruction;
use std::cell::{Ref, RefMut};

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
pub fn margin_trade<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, MarginTrade<'info>>,
    cpi_data: Vec<u8>,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut account = ctx.accounts.account.load_mut()?;
    let active_len = account.indexed_positions.iter_active().count();

    // remaining_accounts layout is expected as follows
    // * active_len number of banks
    // * active_len number of oracles
    // * cpi_program
    // * cpi_accounts

    let banks = &ctx.remaining_accounts[0..active_len];
    let oracles = &ctx.remaining_accounts[active_len..active_len * 2];

    let cpi_program_id = *ctx.remaining_accounts[active_len * 2].key;

    // prepare for cpi
    let (cpi_ais, cpi_ams) = {
        // we also need the group
        let mut cpi_ais = [ctx.accounts.group.to_account_info()].to_vec();
        // skip banks, oracles and cpi program from the remaining_accounts
        let mut remaining_cpi_ais = ctx.remaining_accounts[active_len * 2 + 1..].to_vec();
        cpi_ais.append(&mut remaining_cpi_ais);

        let mut cpi_ams = cpi_ais.to_account_metas(Option::None);
        // we want group to be the signer, so that loans can be taken from the token vaults
        cpi_ams[0].is_signer = true;

        (cpi_ais, cpi_ams)
    };

    // since we are using group signer seeds to invoke cpi,
    // assert that none of the cpi accounts is the mango program to prevent that invoker doesn't
    // abuse this ix to do unwanted changes
    for cpi_ai in &cpi_ais {
        require!(
            cpi_ai.key() != Mango::id(),
            MangoError::InvalidMarginTradeTargetCpiProgram
        );
    }

    // compute pre cpi health
    let pre_cpi_health = compute_health(&mut account, &banks, &oracles)?;
    require!(pre_cpi_health > 0, MangoError::HealthMustBePositive);
    msg!("pre_cpi_health {:?}", pre_cpi_health);

    // prepare and invoke cpi
    let cpi_ix = Instruction {
        program_id: cpi_program_id,
        data: cpi_data,
        accounts: cpi_ams,
    };
    let group_seeds = group_seeds!(group);
    let pre_cpi_token_vault_amounts = get_pre_cpi_token_amounts(&ctx, &cpi_ais);
    solana_program::program::invoke_signed(&cpi_ix, &cpi_ais, &[group_seeds])?;
    adjust_for_post_cpi_token_amounts(
        &ctx,
        &cpi_ais,
        pre_cpi_token_vault_amounts,
        group,
        &mut banks.to_vec(),
        &mut account,
    )?;

    // compute post cpi health
    let post_cpi_health = compute_health(&account, &banks, &oracles)?;
    require!(post_cpi_health > 0, MangoError::HealthMustBePositive);
    msg!("post_cpi_health {:?}", post_cpi_health);

    Ok(())
}

fn get_pre_cpi_token_amounts(ctx: &Context<MarginTrade>, cpi_ais: &Vec<AccountInfo>) -> Vec<u64> {
    let mut mango_vault_token_account_amounts = vec![];
    for maybe_token_account in cpi_ais
        .iter()
        .filter(|ai| ai.owner == &TokenAccount::owner())
    {
        let maybe_mango_vault_token_account =
            Account::<TokenAccount>::try_from(maybe_token_account).unwrap();
        if maybe_mango_vault_token_account.owner == ctx.accounts.group.key() {
            mango_vault_token_account_amounts.push(maybe_mango_vault_token_account.amount)
        }
    }
    mango_vault_token_account_amounts
}

/// withdraws from bank, on users behalf, if he hasn't returned back entire loan amount
fn adjust_for_post_cpi_token_amounts(
    ctx: &Context<MarginTrade>,
    cpi_ais: &Vec<AccountInfo>,
    pre_cpi_token_vault_amounts: Vec<u64>,
    group: Ref<MangoGroup>,
    banks: &mut Vec<AccountInfo>,
    account: &mut RefMut<MangoAccount>,
) -> Result<()> {
    let x = cpi_ais
        .iter()
        .filter(|ai| ai.owner == &TokenAccount::owner());

    for (maybe_token_account, (pre_cpi_token_vault_amount, bank_ai)) in
        util::zip!(x, pre_cpi_token_vault_amounts.iter(), banks.iter())
    {
        let maybe_mango_vault_token_account =
            Account::<TokenAccount>::try_from(maybe_token_account).unwrap();
        if maybe_mango_vault_token_account.owner == ctx.accounts.group.key() {
            let still_loaned_amount =
                pre_cpi_token_vault_amount - maybe_mango_vault_token_account.amount;
            if still_loaned_amount <= 0 {
                continue;
            }

            let token_index = group
                .tokens
                .index_for_mint(&maybe_mango_vault_token_account.mint)?;
            let mut position = *account.indexed_positions.get_mut_or_create(token_index)?;
            let bank_loader = AccountLoader::<'_, TokenBank>::try_from(bank_ai)?;
            let mut bank = bank_loader.load_mut()?;
            // todo: this doesnt work since bank is not mut in the tests atm
            bank.withdraw(&mut position, still_loaned_amount);
        }
    }
    Ok(())
}
