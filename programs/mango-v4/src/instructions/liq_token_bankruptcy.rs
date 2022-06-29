use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::ScanningAccountRetriever;
use crate::state::*;
use crate::util::checked_math as cm;

// Remaining accounts:
// - all banks for liab_token_index (writable)
// - merged health accounts for liqor+liqee
#[derive(Accounts)]
#[instruction(liab_token_index: TokenIndex)]
pub struct LiqTokenBankruptcy<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub liqor: AccountLoader<'info, MangoAccount>,
    #[account(address = liqor.load()?.owner)]
    pub liqor_owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
    )]
    pub liqee: AccountLoader<'info, MangoAccount>,

    #[account(
        has_one = group,
        constraint = liab_mint_info.load()?.token_index == liab_token_index,
    )]
    pub liab_mint_info: AccountLoader<'info, MintInfo>,
}

pub fn liq_token_bankruptcy(
    ctx: Context<LiqTokenBankruptcy>,
    //asset_token_index: TokenIndex,
    liab_token_index: TokenIndex,
    //max_liab_transfer: I80F48,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();

    // split remaining accounts into banks and health
    let liab_mint_info = ctx.accounts.liab_mint_info.load()?;
    let bank_pks = liab_mint_info.banks();
    let (bank_ais, health_ais) = &ctx.remaining_accounts.split_at(bank_pks.len());
    require!(
        bank_ais.iter().map(|ai| ai.key).eq(bank_pks.iter()),
        MangoError::SomeError
    );

    //require!(asset_token_index != liab_token_index, MangoError::SomeError);

    //let mut liqor = ctx.accounts.liqor.load_mut()?;
    //require!(!liqor.is_bankrupt(), MangoError::IsBankrupt);

    let mut liqee = ctx.accounts.liqee.load_mut()?;
    require!(liqee.is_bankrupt(), MangoError::SomeError);

    // find the total deposits
    let mut indexed_total_deposits = I80F48::ZERO;
    for bank_ai in bank_ais.iter() {
        let bank = bank_ai.load::<Bank>()?;
        indexed_total_deposits = cm!(indexed_total_deposits + bank.indexed_deposits);
    }

    let liab_bank = bank_ais[0].load::<Bank>()?;
    let (liqee_position, liqee_raw_token_index, _) =
        liqee.tokens.get_mut_or_create(liab_token_index)?;
    let abs_native_loss = -liqee_position.native(&liab_bank);
    require_gt!(abs_native_loss, I80F48::ZERO);

    // TODO: what if loss is greater than entire deposits?
    // total_indexed_deposits * (deposit_index - new_deposit_index) = abs_native_loss
    let new_deposit_index = cm!(liab_bank.deposit_index - abs_native_loss / indexed_total_deposits);
    drop(liab_bank);

    let mut amount_to_credit = abs_native_loss;
    let mut position_active = false;
    for bank_ai in bank_ais.iter() {
        let mut bank = bank_ai.load_mut::<Bank>()?;
        bank.deposit_index = new_deposit_index;

        // credit liqee on each bank where we can offset borrows
        let amount_for_bank = amount_to_credit.min(bank.native_borrows());
        if amount_for_bank.is_positive() {
            position_active = bank.deposit(liqee_position, amount_for_bank)?;
            amount_to_credit = cm!(amount_to_credit - amount_for_bank);
            if amount_to_credit.is_zero() {
                break;
            }
        }
    }

    // If the account has no more borrows then it's no longer bankrupt
    let account_retriever = ScanningAccountRetriever::new(health_ais, group_pk)?;
    let liqee_health_cache = new_health_cache(&liqee, &account_retriever)?;
    liqee.set_bankrupt(liqee_health_cache.has_borrows());

    // Check liqor's health
    //let liqor_health = compute_health(&liqor, HealthType::Init, &account_retriever)?;
    //require!(liqor_health >= 0, MangoError::HealthMustBePositive);

    require!(!position_active, MangoError::SomeError);
    liqee.tokens.deactivate(liqee_raw_token_index);

    Ok(())
}
