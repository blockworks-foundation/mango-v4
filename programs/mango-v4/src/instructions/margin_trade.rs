use crate::error::MangoError;
use crate::state::{compute_health, Bank, Group, MangoAccount};
use crate::{group_seeds, Mango};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use solana_program::instruction::Instruction;
use std::cell::RefMut;

#[derive(Accounts)]
pub struct MarginTrade<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,
}

pub fn margin_trade<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, MarginTrade<'info>>,
    banks_len: usize,
    cpi_data: Vec<u8>,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut account = ctx.accounts.account.load_mut()?;

    // remaining_accounts layout is expected as follows
    // * banks_len number of banks
    // * banks_len number of oracles
    // * cpi_program
    // * cpi_accounts

    // assert that user has passed in enough banks, this might be greater than his current
    // total number of indexed positions, since
    // user might end up withdrawing or depositing and activating a new indexed position
    require!(
        banks_len >= account.token_account_map.iter_active().count(),
        MangoError::SomeError // todo: SomeError
    );

    // unpack remaining_accounts
    let health_ais = &ctx.remaining_accounts[0..banks_len * 2];
    // TODO: This relies on the particular shape of health_ais
    let banks = &ctx.remaining_accounts[0..banks_len];
    let cpi_program_id = *ctx.remaining_accounts[banks_len * 2].key;

    // prepare account for cpi ix
    let (cpi_ais, cpi_ams) = {
        // we also need the group
        let mut cpi_ais = [ctx.accounts.group.to_account_info()].to_vec();
        // skip banks, oracles and cpi program from the remaining_accounts
        let mut remaining_cpi_ais = ctx.remaining_accounts[banks_len * 2 + 1..].to_vec();
        cpi_ais.append(&mut remaining_cpi_ais);

        // todo: I'm wondering if there's a way to do this without putting cpi_ais on the heap.
        // But fine to defer to the future
        let mut cpi_ams = cpi_ais.to_account_metas(Option::None);
        // we want group to be the signer, so that token vaults can be credited to or withdrawn from
        cpi_ams[0].is_signer = true;

        (cpi_ais, cpi_ams)
    };

    // sanity checks
    for cpi_ai in &cpi_ais {
        // since we are using group signer seeds to invoke cpi,
        // assert that none of the cpi accounts is the mango program to prevent that invoker doesn't
        // abuse this ix to do unwanted changes
        require!(
            cpi_ai.key() != Mango::id(),
            MangoError::InvalidMarginTradeTargetCpiProgram
        );

        // assert that user has passed in the bank for every
        // token account he wants to deposit/withdraw from in cpi
        if cpi_ai.owner == &TokenAccount::owner() {
            let maybe_mango_vault_token_account =
                Account::<TokenAccount>::try_from(cpi_ai).unwrap();
            if maybe_mango_vault_token_account.owner == ctx.accounts.group.key() {
                require!(
                    banks.iter().any(|bank_ai| {
                        let bank_loader = AccountLoader::<'_, Bank>::try_from(bank_ai).unwrap();
                        let bank = bank_loader.load().unwrap();
                        bank.mint == maybe_mango_vault_token_account.mint
                    }),
                    // todo: errorcode
                    MangoError::SomeError
                )
            }
        }
    }

    // compute pre cpi health
    let pre_cpi_health = compute_health(&account, health_ais)?;
    require!(pre_cpi_health > 0, MangoError::HealthMustBePositive);
    msg!("pre_cpi_health {:?}", pre_cpi_health);

    // prepare and invoke cpi
    let cpi_ix = Instruction {
        program_id: cpi_program_id,
        data: cpi_data,
        accounts: cpi_ams,
    };
    let group_seeds = group_seeds!(group);
    let pre_cpi_amounts = get_pre_cpi_amounts(&ctx, &cpi_ais);
    solana_program::program::invoke_signed(&cpi_ix, &cpi_ais, &[group_seeds])?;
    adjust_for_post_cpi_amounts(
        &ctx,
        &cpi_ais,
        pre_cpi_amounts,
        &mut banks.to_vec(),
        &mut account,
    )?;

    // compute post cpi health
    // todo: this is not working, the health is computed on old bank state and not taking into account
    // withdraws done in adjust_for_post_cpi_token_amounts
    let post_cpi_health = compute_health(&account, health_ais)?;
    require!(post_cpi_health > 0, MangoError::HealthMustBePositive);
    msg!("post_cpi_health {:?}", post_cpi_health);

    Ok(())
}

fn get_pre_cpi_amounts(ctx: &Context<MarginTrade>, cpi_ais: &Vec<AccountInfo>) -> Vec<u64> {
    let mut amounts = vec![];
    for token_account in cpi_ais
        .iter()
        .filter(|ai| ai.owner == &TokenAccount::owner())
    {
        let vault = Account::<TokenAccount>::try_from(token_account).unwrap();
        if vault.owner == ctx.accounts.group.key() {
            amounts.push(vault.amount)
        }
    }
    amounts
}

fn adjust_for_post_cpi_amounts(
    ctx: &Context<MarginTrade>,
    cpi_ais: &Vec<AccountInfo>,
    pre_cpi_amounts: Vec<u64>,
    banks: &mut Vec<AccountInfo>,
    account: &mut RefMut<MangoAccount>,
) -> Result<()> {
    let token_accounts_iter = cpi_ais
        .iter()
        .filter(|ai| ai.owner == &TokenAccount::owner());

    for (token_account, pre_cpi_amount) in
        // token_accounts and pre_cpi_amounts are assumed to be in correct order
        token_accounts_iter.zip(pre_cpi_amounts.iter())
    {
        let vault = Account::<TokenAccount>::try_from(token_account).unwrap();
        if vault.owner == ctx.accounts.group.key() {
            // find bank for token account
            let bank_ai = banks
                .iter()
                .find(|bank_ai| {
                    let bank_loader = AccountLoader::<'_, Bank>::try_from(bank_ai).unwrap();
                    let bank = bank_loader.load().unwrap();
                    bank.mint == vault.mint
                })
                .ok_or(MangoError::SomeError)?; // todo: replace SomeError
            let bank_loader = AccountLoader::<'_, Bank>::try_from(bank_ai)?;
            let mut bank = bank_loader.load_mut()?;

            let mut position = *account
                .token_account_map
                .get_mut_or_create(bank.token_index)?
                .0;

            // user has either withdrawn or deposited
            if *pre_cpi_amount > vault.amount {
                bank.withdraw(&mut position, pre_cpi_amount - vault.amount)?;
            } else {
                bank.deposit(&mut position, vault.amount - pre_cpi_amount)?;
            }
        }
    }
    Ok(())
}
