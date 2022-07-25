use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::group_seeds;
use crate::logs::{FlashLoanLog, FlashLoanTokenDetail, TokenBalanceLog};
use crate::state::{
    compute_health, compute_health_from_fixed_accounts, new_fixed_order_account_retriever,
    AccountLoaderDynamic, AccountRetriever, Bank, Group, HealthType, MangoAccount, TokenIndex,
};
use crate::util::checked_math as cm;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_spl::token::{self, Token, TokenAccount};
use fixed::types::I80F48;

/// Sets up mango vaults for flash loan
///
/// In addition to these accounts, there must be a sequence of remaining_accounts:
/// 1. N banks
/// 2. N vaults, matching the banks
#[derive(Accounts)]
pub struct FlashLoan2Begin<'info> {
    pub group: AccountLoader<'info, Group>,
    pub temporary_vault_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,

    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// Finalizes a flash loan
///
/// In addition to these accounts, there must be a sequence of remaining_accounts:
/// 1. health accounts
/// 2. N vaults, matching what was in FlashLoan2Begin
#[derive(Accounts)]
pub struct FlashLoan2End<'info> {
    pub group: AccountLoader<'info, Group>,
    #[account(mut, has_one = group, has_one = owner)]
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn flash_loan2_begin<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoan2Begin<'info>>,
    loan_amounts: Vec<u64>,
) -> Result<()> {
    let num_loans = loan_amounts.len();
    require_eq!(
        ctx.remaining_accounts.len(),
        2 * num_loans,
        MangoError::SomeError
    );
    let banks = &ctx.remaining_accounts[..num_loans];
    let vaults = &ctx.remaining_accounts[num_loans..];

    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);
    let seeds = [&group_seeds[..]];

    // Check that the banks and vaults correspond
    for ((bank_ai, vault_ai), amount) in banks.iter().zip(vaults.iter()).zip(loan_amounts.iter()) {
        let mut bank = bank_ai.load_mut::<Bank>()?;
        require_keys_eq!(bank.group, ctx.accounts.group.key());
        require_keys_eq!(bank.vault, *vault_ai.key);

        let token_account = Account::<TokenAccount>::try_from(vault_ai)?;

        bank.flash_loan_approved_amount = *amount;
        bank.flash_loan_vault_initial = token_account.amount;

        // Approve the withdraw
        if *amount > 0 {
            let approve_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Approve {
                    to: vault_ai.clone(),
                    delegate: ctx.accounts.temporary_vault_authority.to_account_info(),
                    authority: ctx.accounts.group.to_account_info(),
                },
            )
            .with_signer(&seeds);
            token::approve(approve_ctx, *amount)?;
        }
    }

    // Check if the other instructions in the transactions are compatible
    {
        let ixs = ctx.accounts.instructions.as_ref();
        let current_index = tx_instructions::load_current_index_checked(ixs)? as usize;

        // Forbid FlashLoan2Begin to be called from CPI (it does not have to be the first instruction)
        let current_ix = tx_instructions::load_instruction_at_checked(current_index, ixs)?;
        require_keys_eq!(
            current_ix.program_id,
            *ctx.program_id,
            MangoError::SomeError
        );

        // The only other mango instruction that must appear before the end of the tx is
        // the FlashLoan2End instruction. No other mango instructions are allowed.
        let mut index = current_index + 1;
        let mut found_end = false;
        loop {
            let ix = match tx_instructions::load_instruction_at_checked(index, ixs) {
                Ok(ix) => ix,
                Err(ProgramError::InvalidArgument) => break, // past the last instruction
                Err(e) => Err(e)?,
            };

            // Check that the mango program key is not used
            if ix.program_id == crate::id() {
                // must be the last mango ix -- this could possibly be relaxed, but right now
                // we need to guard against multiple FlashLoanEnds
                require!(!found_end, MangoError::SomeError);
                found_end = true;

                // must be the FlashLoan2End instruction
                require!(
                    &ix.data[0..8] == &[187, 107, 239, 212, 18, 21, 145, 171],
                    MangoError::SomeError
                );

                // check that the same vaults are passed
                let end_vaults = &ix.accounts[ix.accounts.len() - num_loans..];
                for (start_vault, end_vault) in vaults.iter().zip(end_vaults.iter()) {
                    require_keys_eq!(*start_vault.key, end_vault.pubkey);
                }
            } else {
                // ensure no one can cpi into mango either
                for meta in ix.accounts.iter() {
                    require_keys_neq!(meta.pubkey, crate::id());
                }
            }

            index += 1;
        }
        require!(found_end, MangoError::SomeError);
    }

    Ok(())
}

struct TokenVaultChange {
    bank_index: usize,
    raw_token_index: usize,
    amount: I80F48,
}

// Remaining accounts:
// 1. health
// 2. vaults (must be same as FlashLoanStart)
pub fn flash_loan2_end<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoan2End<'info>>,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);

    let mut account = ctx.accounts.account.load_mut()?;

    require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);
    // Find index at which vaults start
    let vaults_index = ctx
        .remaining_accounts
        .iter()
        .position(|ai| {
            let maybe_token_account = Account::<TokenAccount>::try_from(ai);
            if maybe_token_account.is_err() {
                return false;
            }

            maybe_token_account.unwrap().owner == account.fixed.group
        })
        .ok_or_else(|| error!(MangoError::SomeError))?;

    // First initialize to the remaining delegated amount
    let health_ais = &ctx.remaining_accounts[..vaults_index];
    let vaults = &ctx.remaining_accounts[vaults_index..];
    let mut vaults_with_banks = vec![false; vaults.len()];

    // Loop over the banks, finding matching vaults
    // TODO: must be moved into health.rs, because it assumes something about the health accounts structure
    let mut changes = vec![];
    for (i, bank_ai) in health_ais.iter().enumerate() {
        // iterate until the first non-bank
        let bank = match bank_ai.load::<Bank>() {
            Ok(b) => b,
            Err(_) => break,
        };

        // find a vault -- if there's none, skip
        let (vault_index, vault_ai) = match vaults
            .iter()
            .enumerate()
            .find(|(_, vault_ai)| vault_ai.key == &bank.vault)
        {
            Some(v) => v,
            None => continue,
        };

        let vault = Account::<TokenAccount>::try_from(vault_ai)?;
        vaults_with_banks[vault_index] = true;

        // Ensure this bank/vault combination was mentioned in the Begin instruction:
        // The Begin instruction only checks that End ends with the same vault accounts -
        // but there could be an extra vault account in End, or a different bank could be
        // used for the same vault.
        require_neq!(bank.flash_loan_vault_initial, u64::MAX);

        // Create the token position now, so we can compute the pre-health with fixed order health accounts
        let (_, raw_token_index, _) = account.token_get_mut_or_create(bank.token_index)?;

        // Revoke delegation
        let ix = token::spl_token::instruction::revoke(
            &token::spl_token::ID,
            vault_ai.key,
            &ctx.accounts.group.key(),
            &[],
        )?;
        solana_program::program::invoke_signed(
            &ix,
            &[vault_ai.clone(), ctx.accounts.group.to_account_info()],
            &[group_seeds],
        )?;

        // Track vault difference
        let new_amount = I80F48::from(vault.amount);
        let old_amount = I80F48::from(bank.flash_loan_vault_initial);
        let change = cm!(new_amount - old_amount);

        changes.push(TokenVaultChange {
            bank_index: i,
            raw_token_index,
            amount: change,
        });
    }

    // all vaults must have had matching banks
    require!(vaults_with_banks.iter().all(|&b| b), MangoError::SomeError);

    // Check pre-cpi health
    // NOTE: This health check isn't strictly necessary. It will be, later, when
    // we want to have reduce_only or be able to move an account out of bankruptcy.
    let retriever = new_fixed_order_account_retriever(health_ais, &account.borrow())?;
    let pre_cpi_health = compute_health(&account.borrow(), HealthType::Init, &retriever)?;
    require!(pre_cpi_health >= 0, MangoError::HealthMustBePositive);
    msg!("pre_cpi_health {:?}", pre_cpi_health);

    // Prices for logging
    let mut prices = vec![];
    for change in &changes {
        let (_, oracle_price) = retriever.bank_and_oracle(
            &account.fixed.group,
            change.bank_index,
            change.raw_token_index as TokenIndex,
        )?;

        prices.push(oracle_price);
    }
    // Drop retriever as mut bank below uses health_ais
    drop(retriever);

    // Apply the vault diffs to the bank positions
    let mut deactivated_token_positions = vec![];
    let mut token_loan_details = Vec::with_capacity(changes.len());
    for (change, price) in changes.iter().zip(prices.iter()) {
        let mut bank = health_ais[change.bank_index].load_mut::<Bank>()?;
        let position = account.token_get_mut_raw(change.raw_token_index);
        let native = position.native(&bank);
        let approved_amount = I80F48::from(bank.flash_loan_approved_amount);

        let loan = if native.is_positive() {
            cm!(approved_amount - native).max(I80F48::ZERO)
        } else {
            approved_amount
        };

        let loan_origination_fee = cm!(loan * bank.loan_origination_fee_rate);
        bank.collected_fees_native = cm!(bank.collected_fees_native + loan_origination_fee);

        let is_active =
            bank.change_without_fee(position, cm!(change.amount - loan_origination_fee))?;
        if !is_active {
            deactivated_token_positions.push(change.raw_token_index);
        }

        bank.flash_loan_approved_amount = 0;
        bank.flash_loan_vault_initial = u64::MAX;

        token_loan_details.push(FlashLoanTokenDetail {
            token_index: position.token_index,
            change_amount: change.amount.to_bits(),
            loan: loan.to_bits(),
            loan_origination_fee: loan_origination_fee.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
            price: price.to_bits(),
        });

        emit!(TokenBalanceLog {
            mango_account: ctx.accounts.account.key(),
            token_index: bank.token_index as u16,
            indexed_position: position.indexed_position.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
            price: price.to_bits(),
        });
    }

    emit!(FlashLoanLog {
        mango_account: ctx.accounts.account.key(),
        token_loan_details
    });

    // Check post-cpi health
    let post_cpi_health =
        compute_health_from_fixed_accounts(&account.borrow(), HealthType::Init, health_ais)?;
    require!(post_cpi_health >= 0, MangoError::HealthMustBePositive);
    msg!("post_cpi_health {:?}", post_cpi_health);

    // Deactivate inactive token accounts after health check
    for raw_token_index in deactivated_token_positions {
        account.token_deactivate(raw_token_index);
    }

    Ok(())
}
