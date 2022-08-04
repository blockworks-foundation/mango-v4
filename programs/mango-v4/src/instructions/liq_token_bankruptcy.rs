use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
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
    #[account(
        has_one = insurance_vault,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub liqor: AccountLoaderDynamic<'info, MangoAccount>,
    pub liqor_owner: Signer<'info>,

    #[account(mut, has_one = group)]
    pub liqee: AccountLoaderDynamic<'info, MangoAccount>,

    #[account(
        has_one = group,
        constraint = liab_mint_info.load()?.token_index == liab_token_index,
    )]
    pub liab_mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub quote_vault: Account<'info, TokenAccount>,

    // future: this would be an insurance fund vault specific to a
    // trustless token, separate from the shared one on the group
    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> LiqTokenBankruptcy<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.insurance_vault.to_account_info(),
            to: self.quote_vault.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

pub fn liq_token_bankruptcy(
    ctx: Context<LiqTokenBankruptcy>,
    liab_token_index: TokenIndex,
    max_liab_transfer: I80F48,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_pk = &ctx.accounts.group.key();

    // split remaining accounts into banks and health
    let liab_mint_info = ctx.accounts.liab_mint_info.load()?;
    let bank_pks = liab_mint_info.banks();
    let (bank_ais, health_ais) = &ctx.remaining_accounts.split_at(bank_pks.len());
    require!(
        bank_ais.iter().map(|ai| ai.key).eq(bank_pks.iter()),
        MangoError::SomeError
    );

    let mut liqor = ctx.accounts.liqor.load_mut()?;
    require!(
        liqor
            .fixed
            .is_owner_or_delegate(ctx.accounts.liqor_owner.key()),
        MangoError::SomeError
    );
    require!(!liqor.fixed.is_bankrupt(), MangoError::IsBankrupt);

    let mut liqee = ctx.accounts.liqee.load_mut()?;
    require!(liqee.fixed.is_bankrupt(), MangoError::IsBankrupt);

    let liab_bank = bank_ais[0].load::<Bank>()?;
    let liab_deposit_index = liab_bank.deposit_index;
    let (liqee_liab, liqee_raw_token_index) = liqee.token_get_mut(liab_token_index)?;
    let mut remaining_liab_loss = -liqee_liab.native(&liab_bank);
    require_gt!(remaining_liab_loss, I80F48::ZERO);
    drop(liab_bank);

    let mut account_retriever = ScanningAccountRetriever::new(health_ais, group_pk)?;

    // find insurance transfer amount
    let (liab_bank, liab_price, opt_quote_bank_and_price) =
        account_retriever.banks_mut_and_oracles(liab_token_index, QUOTE_TOKEN_INDEX)?;
    let liab_fee_factor = if liab_token_index == QUOTE_TOKEN_INDEX {
        I80F48::ONE
    } else {
        cm!(I80F48::ONE + liab_bank.liquidation_fee)
    };
    let liab_price_adjusted = cm!(liab_price * liab_fee_factor);

    let liab_transfer_unrounded = remaining_liab_loss.min(max_liab_transfer);

    let insurance_vault_amount = if liab_mint_info.elligible_for_group_insurance_fund() {
        ctx.accounts.insurance_vault.amount
    } else {
        0
    };

    let insurance_transfer = cm!(liab_transfer_unrounded * liab_price_adjusted)
        .checked_ceil()
        .unwrap()
        .checked_to_num::<u64>()
        .unwrap()
        .min(insurance_vault_amount);

    let insurance_fund_exhausted = insurance_transfer == insurance_vault_amount;

    let insurance_transfer_i80f48 = I80F48::from(insurance_transfer);

    // AUDIT: v3 does this, but it seems bad, because it can make liab_transfer
    // exceed max_liab_transfer due to the ceil() above! Otoh, not doing it would allow
    // liquidators to exploit the insurance fund for 1 native token each call.
    let liab_transfer = cm!(insurance_transfer_i80f48 / liab_price_adjusted);

    let mut liqee_liab_active = true;
    if insurance_transfer > 0 {
        // in the end, the liqee gets liab assets
        liqee_liab_active = liab_bank.deposit(liqee_liab, liab_transfer)?;
        remaining_liab_loss = -liqee_liab.native(liab_bank);

        // move insurance assets into quote bank
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            insurance_transfer,
        )?;

        // move quote assets into liqor and withdraw liab assets
        if let Some((quote_bank, _)) = opt_quote_bank_and_price {
            require_keys_eq!(quote_bank.vault, ctx.accounts.quote_vault.key());
            require_keys_eq!(quote_bank.mint, ctx.accounts.insurance_vault.mint);

            // credit the liqor
            let (liqor_quote, liqor_quote_raw_token_index, _) =
                liqor.token_get_mut_or_create(QUOTE_TOKEN_INDEX)?;
            let liqor_quote_active = quote_bank.deposit(liqor_quote, insurance_transfer_i80f48)?;

            // transfer liab from liqee to liqor
            let (liqor_liab, liqor_liab_raw_token_index, _) =
                liqor.token_get_mut_or_create(liab_token_index)?;
            let liqor_liab_active = liab_bank.withdraw_with_fee(liqor_liab, liab_transfer)?;

            // Check liqor's health
            let liqor_health =
                compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)?;
            require!(liqor_health >= 0, MangoError::HealthMustBePositive);

            if !liqor_quote_active {
                liqor.token_deactivate(liqor_quote_raw_token_index);
            }
            if !liqor_liab_active {
                liqor.token_deactivate(liqor_liab_raw_token_index);
            }
        } else {
            // For liab_token_index == QUOTE_TOKEN_INDEX: the insurance fund deposits directly into liqee,
            // without a fee or the liqor being involved
            require_eq!(liab_token_index, QUOTE_TOKEN_INDEX);
            require_eq!(liab_price_adjusted, I80F48::ONE);
            require_eq!(insurance_transfer_i80f48, liab_transfer);
        }
    }
    drop(account_retriever);

    // Socialize loss
    if insurance_fund_exhausted && remaining_liab_loss.is_positive() {
        // find the total deposits
        let mut indexed_total_deposits = I80F48::ZERO;
        for bank_ai in bank_ais.iter() {
            let bank = bank_ai.load::<Bank>()?;
            indexed_total_deposits = cm!(indexed_total_deposits + bank.indexed_deposits);
        }

        // This is the solution to:
        //   total_indexed_deposits * (deposit_index - new_deposit_index) = remaining_liab_loss
        // AUDIT: Could it happen that remaining_liab_loss > total_indexed_deposits * deposit_index?
        //        Probably not.
        let new_deposit_index =
            cm!(liab_deposit_index - remaining_liab_loss / indexed_total_deposits);

        let mut amount_to_credit = remaining_liab_loss;
        let mut position_active = true;
        for bank_ai in bank_ais.iter() {
            let mut bank = bank_ai.load_mut::<Bank>()?;
            bank.deposit_index = new_deposit_index;

            // credit liqee on each bank where we can offset borrows
            let amount_for_bank = amount_to_credit.min(bank.native_borrows());
            if amount_for_bank.is_positive() {
                position_active = bank.deposit(liqee_liab, amount_for_bank)?;
                amount_to_credit = cm!(amount_to_credit - amount_for_bank);
                if amount_to_credit.is_zero() {
                    break;
                }
            }
        }
        require!(!position_active, MangoError::SomeError);
        liqee_liab_active = false;
    }

    // If the account has no more borrows then it's no longer bankrupt
    // and should (always?) no longer be liquidated.
    let account_retriever = ScanningAccountRetriever::new(health_ais, group_pk)?;
    let liqee_health_cache = new_health_cache(&liqee.borrow(), &account_retriever)?;
    liqee.fixed.set_bankrupt(liqee_health_cache.has_borrows());
    if !liqee.is_bankrupt() && liqee_health_cache.health(HealthType::Init) >= 0 {
        liqee.fixed.set_being_liquidated(false);
    }

    if !liqee_liab_active {
        liqee.token_deactivate(liqee_raw_token_index);
    }

    Ok(())
}
