use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::group_seeds;
use crate::health::{new_fixed_order_account_retriever, new_health_cache, AccountRetriever};
use crate::logs::{FlashLoanLog, FlashLoanTokenDetail, TokenBalanceLog};
use crate::state::*;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_lang::Discriminator;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, TokenAccount};
use fixed::types::I80F48;

/// The `loan_amounts` argument lists the amount to be loaned from each bank/vault and
/// the order matches the order of bank accounts.
pub fn flash_loan_begin<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanBegin<'info>>,
    loan_amounts: Vec<u64>,
) -> Result<()> {
    let account = ctx.accounts.account.load_full_mut()?;

    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let num_loans = loan_amounts.len();
    require_eq!(ctx.remaining_accounts.len(), 3 * num_loans + 1);
    let banks = &ctx.remaining_accounts[..num_loans];
    let vaults = &ctx.remaining_accounts[num_loans..2 * num_loans];
    let token_accounts = &ctx.remaining_accounts[2 * num_loans..3 * num_loans];
    let group_ai = &ctx.remaining_accounts[3 * num_loans];

    let group_al = AccountLoader::<Group>::try_from(group_ai)?;
    let group = group_al.load()?;
    require!(
        group.is_ix_enabled(IxGate::FlashLoan),
        MangoError::IxIsDisabled
    );

    let group_seeds = group_seeds!(group);
    let seeds = [&group_seeds[..]];

    // This instruction does not currently deal with:
    // - borrowing twice from the same bank
    // - borrowing from two different banks for the same token
    // Hence we collect all token_indexes and ensure each appears only once.
    let mut seen_token_indexes = Vec::with_capacity(num_loans);

    // Check that the banks and vaults correspond
    for (((bank_ai, vault_ai), token_account_ai), amount) in banks
        .iter()
        .zip(vaults.iter())
        .zip(token_accounts.iter())
        .zip(loan_amounts.iter())
    {
        let mut bank = bank_ai.load_mut::<Bank>()?;
        require_keys_eq!(bank.group, group_ai.key());
        require_keys_eq!(bank.vault, *vault_ai.key);

        require_msg!(
            !seen_token_indexes.contains(&bank.token_index),
            "each loan must be for a unique token_index"
        );
        seen_token_indexes.push(bank.token_index);

        let vault = Account::<TokenAccount>::try_from(vault_ai)?;
        let token_account = Account::<TokenAccount>::try_from(token_account_ai)?;

        require_keys_neq!(token_account.owner, group_ai.key());

        bank.flash_loan_approved_amount = *amount;
        bank.flash_loan_token_account_initial = token_account.amount;

        // Transfer the loaned funds
        if *amount > 0 {
            // Provide a readable error message in case the vault doesn't have enough tokens
            if vault.amount < *amount {
                return err!(MangoError::InsufficentBankVaultFunds).with_context(|| {
                    format!(
                        "bank vault {} does not have enough tokens, need {} but have {}",
                        vault_ai.key, amount, vault.amount
                    )
                });
            }

            let transfer_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: vault_ai.clone(),
                    to: token_account_ai.clone(),
                    authority: group_ai.clone(),
                },
            )
            .with_signer(&seeds);
            token::transfer(transfer_ctx, *amount)?;
        }
    }

    // Check if the other instructions in the transactions are compatible
    {
        let ixs = ctx.accounts.instructions.as_ref();
        let current_index = tx_instructions::load_current_index_checked(ixs)? as usize;

        // Forbid FlashLoanBegin to be called from CPI (it does not have to be the first instruction)
        let current_ix = tx_instructions::load_instruction_at_checked(current_index, ixs)?;
        require_msg!(
            current_ix.program_id == *ctx.program_id,
            "FlashLoanBegin must be a top-level instruction"
        );

        // The only other mango instruction that must appear before the end of the tx is
        // the FlashLoanEnd instruction. No other mango instructions are allowed.
        let mut index = current_index + 1;
        let mut found_end = false;
        loop {
            let ix = match tx_instructions::load_instruction_at_checked(index, ixs) {
                Ok(ix) => ix,
                Err(ProgramError::InvalidArgument) => break, // past the last instruction
                Err(e) => return Err(e.into()),
            };

            if account.fixed.is_delegate(ctx.accounts.owner.key()) {
                require_msg!(
                    ix.program_id == AssociatedToken::id()
                        || ix.program_id == jupiter_mainnet_3::ID
                        || ix.program_id == jupiter_mainnet_4::ID,
                    "delegate is only allowed to pass in ixs to ATA or Jupiter v3 or v4 programs"
                );
            }

            // Check that the mango program key is not used
            if ix.program_id == crate::id() {
                // must be the last mango ix -- this could possibly be relaxed, but right now
                // we need to guard against multiple FlashLoanEnds
                require_msg!(
                    !found_end,
                    "the transaction must not contain a Mango instruction after FlashLoanEnd"
                );
                found_end = true;

                // must be the FlashLoanEnd instruction
                require!(
                    ix.data[0..8] == crate::instruction::FlashLoanEnd::discriminator(),
                    MangoError::SomeError
                );

                require_msg!(
                    ctx.accounts.account.key() == ix.accounts[0].pubkey,
                    "the mango account passed to FlashLoanBegin and End must match"
                );

                // check that the same vaults and token accounts are passed
                let begin_accounts = &ctx.remaining_accounts[num_loans..];
                let end_accounts = &ix.accounts[ix.accounts.len() - begin_accounts.len()..];
                for (begin_account, end_account) in begin_accounts.iter().zip(end_accounts.iter()) {
                    require_msg!(*begin_account.key == end_account.pubkey, "the trailing vault, token and group accounts passed to FlashLoanBegin and End must match, found {} on begin and {} on end", begin_account.key, end_account.pubkey);
                }
            } else {
                // ensure no one can cpi into mango either
                for meta in ix.accounts.iter() {
                    require_msg!(meta.pubkey != crate::id(), "instructions between FlashLoanBegin and End may not use the Mango program account");
                }
            }

            index += 1;
        }
        require_msg!(
            found_end,
            "found no FlashLoanEnd instruction in transaction"
        );
    }

    Ok(())
}

struct TokenVaultChange {
    token_index: TokenIndex,
    bank_index: usize,
    raw_token_index: usize,
    amount: I80F48,
}

pub fn flash_loan_end<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanEnd<'info>>,
    flash_loan_type: FlashLoanType,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;

    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
        MangoError::SomeError
    );

    let group = account.fixed.group;

    let remaining_len = ctx.remaining_accounts.len();
    let group_ai = &ctx.remaining_accounts[remaining_len - 1];
    require_keys_eq!(group, group_ai.key());

    // Find index at which vaults start
    let vaults_len = ctx.remaining_accounts[..remaining_len - 1]
        .iter()
        .rev()
        .map_while(|ai| Account::<TokenAccount>::try_from(ai).ok())
        .position(|token_account| token_account.owner == group)
        .ok_or_else(|| {
            error_msg!("expected at least one group-owned vault token account to be passed")
        })?;
    let vaults_index = remaining_len - 2 * vaults_len - 1;

    let health_ais = &ctx.remaining_accounts[..vaults_index];
    let vaults = &ctx.remaining_accounts[vaults_index..vaults_index + vaults_len];
    let token_accounts =
        &ctx.remaining_accounts[vaults_index + vaults_len..vaults_index + 2 * vaults_len];

    // Verify that each mentioned vault has a bank in the health accounts
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
        require_keys_eq!(bank.group, group);

        // find a vault -- if there's none, skip
        let (vault_index, vault_ai) = match vaults
            .iter()
            .enumerate()
            .find(|(_, vault_ai)| vault_ai.key == &bank.vault)
        {
            Some(v) => v,
            None => continue,
        };

        vaults_with_banks[vault_index] = true;
        let token_account_ai = &token_accounts[vault_index];
        let token_account = Account::<TokenAccount>::try_from(token_account_ai)?;

        // Ensure this bank/vault combination was mentioned in the Begin instruction:
        // The Begin instruction only checks that End ends with the same vault accounts -
        // but there could be an extra vault account in End, or a different bank could be
        // used for the same vault.
        require_neq!(bank.flash_loan_token_account_initial, u64::MAX);

        // Create the token position now, so we can compute the pre-health with fixed order health accounts
        let (_, raw_token_index, _) = account.ensure_token_position(bank.token_index)?;

        // Transfer any excess over the inital balance of the token account back
        // into the vault. Compute the total change in the vault balance.
        let mut change = -I80F48::from(bank.flash_loan_approved_amount);
        if token_account.amount > bank.flash_loan_token_account_initial {
            let transfer_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: token_account_ai.clone(),
                    to: vault_ai.clone(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            );
            let repay = token_account.amount - bank.flash_loan_token_account_initial;
            token::transfer(transfer_ctx, repay)?;

            let repay = I80F48::from(repay);
            change += repay;
        }

        changes.push(TokenVaultChange {
            token_index: bank.token_index,
            bank_index: i,
            raw_token_index,
            amount: change,
        });
    }

    match flash_loan_type {
        FlashLoanType::Unknown => {}
        FlashLoanType::Swap => {
            require_msg!(
                changes.len() == 2,
                "when flash_loan_type is Swap there must be exactly 2 token vault changes"
            )
        }
    }

    // all vaults must have had matching banks
    for (i, has_bank) in vaults_with_banks.iter().enumerate() {
        require_msg!(
            has_bank,
            "missing bank for vault index {}, address {}",
            i,
            vaults[i].key
        );
    }

    // Check health before balance adjustments
    let retriever = new_fixed_order_account_retriever(health_ais, &account.borrow())?;
    let health_cache = new_health_cache(&account.borrow(), &retriever)?;
    let pre_init_health = account.check_health_pre(&health_cache)?;

    // Prices for logging and net borrow checks
    let mut oracle_prices = vec![];
    for change in &changes {
        let (_, oracle_price) = retriever.bank_and_oracle(
            &account.fixed.group,
            change.bank_index,
            change.token_index,
        )?;

        oracle_prices.push(oracle_price);
    }
    // Drop retriever as mut bank below uses health_ais
    drop(retriever);

    // Apply the vault diffs to the bank positions
    let mut deactivated_token_positions = vec![];
    let mut token_loan_details = Vec::with_capacity(changes.len());
    for (change, oracle_price) in changes.iter().zip(oracle_prices.iter()) {
        let mut bank = health_ais[change.bank_index].load_mut::<Bank>()?;

        let position = account.token_position_mut_by_raw_index(change.raw_token_index);
        let native = position.native(&bank);

        let approved_amount = I80F48::from(bank.flash_loan_approved_amount);

        let loan = if native.is_positive() {
            (approved_amount - native).max(I80F48::ZERO)
        } else {
            approved_amount
        };

        let loan_origination_fee = loan * bank.loan_origination_fee_rate;
        bank.collected_fees_native += loan_origination_fee;

        let change_amount = change.amount - loan_origination_fee;
        let native_after_change = native + change_amount;
        if bank.is_reduce_only() {
            require!(
                (change_amount < 0 && native_after_change >= 0)
                    || (change_amount > 0 && native_after_change < 1),
                MangoError::TokenInReduceOnlyMode
            );
        }

        // Enforce min vault to deposits ratio
        if native_after_change < 0 {
            let vault_ai = vaults
                .iter()
                .find(|vault_ai| vault_ai.key == &bank.vault)
                .unwrap();
            bank.enforce_min_vault_to_deposits_ratio(vault_ai)?;
        }

        let is_active = bank.change_without_fee(
            position,
            change.amount - loan_origination_fee,
            Clock::get()?.unix_timestamp.try_into().unwrap(),
            *oracle_price,
        )?;
        if !is_active {
            deactivated_token_positions.push(change.raw_token_index);
        }

        bank.flash_loan_approved_amount = 0;
        bank.flash_loan_token_account_initial = u64::MAX;

        token_loan_details.push(FlashLoanTokenDetail {
            token_index: position.token_index,
            change_amount: change.amount.to_bits(),
            loan: loan.to_bits(),
            loan_origination_fee: loan_origination_fee.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
            price: oracle_price.to_bits(),
        });

        emit!(TokenBalanceLog {
            mango_group: group.key(),
            mango_account: ctx.accounts.account.key(),
            token_index: bank.token_index as u16,
            indexed_position: position.indexed_position.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
        });
    }

    emit!(FlashLoanLog {
        mango_group: group.key(),
        mango_account: ctx.accounts.account.key(),
        flash_loan_type,
        token_loan_details
    });

    // Check health after account position changes
    let retriever = new_fixed_order_account_retriever(health_ais, &account.borrow())?;
    let health_cache = new_health_cache(&account.borrow(), &retriever)?;
    account.check_health_post(&health_cache, pre_init_health)?;

    // Deactivate inactive token accounts after health check
    for raw_token_index in deactivated_token_positions {
        account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
    }

    Ok(())
}
