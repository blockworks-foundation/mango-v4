use crate::accounts_ix::*;
use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::group_seeds;
use crate::health::{new_fixed_order_account_retriever, new_health_cache, AccountRetriever};
use crate::logs::{emit_stack, FlashLoanLogV3, FlashLoanTokenDetailV3, TokenBalanceLog};
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
    program_id: &Pubkey,
    account_ai: &AccountLoader<'info, MangoAccountFixed>,
    owner_pk: &Pubkey,
    instructions_ai: &AccountInfo<'info>,
    token_program_ai: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    loan_amounts: Vec<u64>,
) -> Result<()> {
    let num_loans = loan_amounts.len();
    require_gt!(num_loans, 0);

    // Loans of 0 are acceptable and common: Users often want to loan some of token A,
    // nothing of token B, swap A to B and then deposit the gains.

    let account = account_ai.load_full_mut()?;

    // account constraint #1
    require!(
        account.fixed.is_owner_or_delegate(*owner_pk),
        MangoError::SomeError
    );

    require_eq!(remaining_accounts.len(), 3 * num_loans + 1);
    let banks = &remaining_accounts[..num_loans];
    let vaults = &remaining_accounts[num_loans..2 * num_loans];
    let token_accounts = &remaining_accounts[2 * num_loans..3 * num_loans];
    let group_ai = &remaining_accounts[3 * num_loans];

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

        require_keys_eq!(token_account.mint, bank.mint);

        // This check is likely unnecessary
        require_keys_neq!(token_account.owner, group_ai.key());

        require_eq!(bank.flash_loan_approved_amount, 0);
        require_eq!(bank.flash_loan_token_account_initial, u64::MAX);
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
                token_program_ai.clone(),
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
        let ixs = instructions_ai;
        let current_index = tx_instructions::load_current_index_checked(ixs)? as usize;

        // Forbid FlashLoanBegin to be called from CPI (it does not have to be the first instruction)
        let current_ix = tx_instructions::load_instruction_at_checked(current_index, ixs)?;
        require_msg!(
            &current_ix.program_id == program_id,
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

            if account.fixed.is_delegate(*owner_pk) {
                require_msg!(
                    ix.program_id == AssociatedToken::id()
                        || ix.program_id == jupiter_mainnet_3::ID
                        || ix.program_id == jupiter_mainnet_4::ID
                        || ix.program_id == jupiter_mainnet_6::ID
                        || ix.program_id == compute_budget::ID
                        || ix.program_id == crate::id(),
                    "delegate is only allowed to pass in ixs to ATA or Jupiter v3/v4/v6 programs, passed ({})", ix.program_id
                );
            }

            // Check that the mango program key is not used
            if ix.program_id == crate::id() {
                // must be the FlashLoanEnd instruction
                require!(
                    ix.data[0..8] == crate::instruction::FlashLoanEndV2::discriminator(),
                    MangoError::SomeError
                );
                // the correct number of loans is passed to the End instruction
                require_eq!(ix.data[8] as usize, num_loans);

                require_msg!(
                    account_ai.key() == ix.accounts[0].pubkey,
                    "the mango account passed to FlashLoanBegin and End must match"
                );

                // check that the same vaults and token accounts are passed
                let begin_accounts = &remaining_accounts[num_loans..];
                let end_accounts = &ix.accounts[ix.accounts.len() - begin_accounts.len()..];
                for (begin_account, end_account) in begin_accounts.iter().zip(end_accounts.iter()) {
                    require_msg!(*begin_account.key == end_account.pubkey, "the trailing vault, token and group accounts passed to FlashLoanBegin and End must match, found {} on begin and {} on end", begin_account.key, end_account.pubkey);
                }

                // No need to check any instructions after the end instruction.
                // "Duplicate FlashLoanEnd" is guarded against the same way as "End without Begin":
                // The End instruction requires at least one bank-vault pair and that bank
                // must have flash_loan_token_account_initial set - which only happens in Begin.
                found_end = true;
                break;
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

pub fn flash_loan_swap_begin<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, FlashLoanSwapBegin<'info>>,
    loan_amount: u64,
) -> Result<()> {
    // Create missing token accounts if needed. We do this here because
    // it uses up fewer tx bytes than emitting the two create-idempotent instructions
    // separately. Primarily because top-level ix program addresses can't be in
    // an address lookup table.

    // Remaining accounts are banks, vaults, token accounts, group
    let rlen = ctx.remaining_accounts.len();
    require_eq!(rlen, 2 + 2 + 2 + 1);
    {
        let input_account = &ctx.remaining_accounts[rlen - 3];
        let ctx = CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.owner.to_account_info(),
                associated_token: input_account.clone(),
                authority: ctx.accounts.owner.to_account_info(),
                mint: ctx.accounts.input_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        );
        anchor_spl::associated_token::create_idempotent(ctx)?;
    }
    {
        let output_account = &ctx.remaining_accounts[rlen - 2];
        let ctx = CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.owner.to_account_info(),
                associated_token: output_account.clone(),
                authority: ctx.accounts.owner.to_account_info(),
                mint: ctx.accounts.output_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        );
        anchor_spl::associated_token::create_idempotent(ctx)?;
    }

    flash_loan_begin(
        ctx.program_id,
        &ctx.accounts.account,
        ctx.accounts.owner.key,
        &ctx.accounts.instructions,
        &ctx.accounts.token_program,
        ctx.remaining_accounts,
        vec![loan_amount, 0],
    )?;
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
    num_loans: u8,
    flash_loan_type: FlashLoanType,
) -> Result<()> {
    require_gt!(num_loans, 0);

    // FlashLoanEnd can only be called in the same tx as a FlashLoanBegin because:
    // - FlashLoanBegin checks for a matching FlashLoanEnd in the same tx
    // - FlashLoanBegin sets flash_loan_token_account_initial on a bank, which is
    //   validated below. (and there must be at least one bank-vault-token account triple)

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
    let vaults_len: usize = num_loans.into();
    let vaults_index = remaining_len - 2 * vaults_len - 1;

    let health_ais = &ctx.remaining_accounts[..vaults_index];
    let vaults = &ctx.remaining_accounts[vaults_index..vaults_index + vaults_len];
    let token_accounts =
        &ctx.remaining_accounts[vaults_index + vaults_len..vaults_index + 2 * vaults_len];

    // Verify that each mentioned vault has a bank in the health accounts
    let mut vaults_with_banks = vec![false; vaults.len()];

    // Biggest flash_loan_swap_fee_rate over all involved banks
    let mut max_swap_fee_rate = 0.0f32;

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

        // The token account could have been re-initialized for a different mint
        require_keys_eq!(token_account.mint, bank.mint);

        // Ensure this bank/vault combination was mentioned in the Begin instruction:
        // The Begin instruction only checks that End ends with the same vault accounts -
        // but there could be an extra vault account in End, or a different bank could be
        // used for the same vault.
        // This check guarantees that FlashLoanBegin was called on this bank.
        require_neq!(bank.flash_loan_token_account_initial, u64::MAX);

        // Create the token position now, so we can compute the pre-health with fixed order health accounts
        let (_, raw_token_index, _) = account.ensure_token_position(bank.token_index)?;

        // Transfer any excess over the initial balance of the token account back
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

        max_swap_fee_rate = max_swap_fee_rate.max(bank.flash_loan_swap_fee_rate);

        changes.push(TokenVaultChange {
            token_index: bank.token_index,
            bank_index: i,
            raw_token_index,
            amount: change,
        });
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

    match flash_loan_type {
        FlashLoanType::Unknown => {}
        FlashLoanType::Swap | FlashLoanType::SwapWithoutFee => {
            require_msg!(
                changes.len() == 2,
                "when flash_loan_type is Swap or SwapWithoutFee there must be exactly 2 token vault changes"
            )
        }
    }

    // Check health before balance adjustments
    let retriever = new_fixed_order_account_retriever(health_ais, &account.borrow())?;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)?;
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

        let approved_amount_u64 = bank.flash_loan_approved_amount;
        let approved_amount = I80F48::from(approved_amount_u64);

        let loan = if native.is_positive() {
            (approved_amount - native).max(I80F48::ZERO)
        } else {
            approved_amount
        };

        let loan_origination_fee = loan * bank.loan_origination_fee_rate;
        bank.collected_fees_native += loan_origination_fee;

        let swap_fee = if change.amount < 0 && flash_loan_type == FlashLoanType::Swap {
            -change.amount * I80F48::from_num(max_swap_fee_rate)
        } else {
            I80F48::ZERO
        };
        bank.collected_fees_native += swap_fee;

        let change_amount = change.amount - loan_origination_fee - swap_fee;
        let native_after_change = native + change_amount;
        if bank.are_deposits_reduce_only() {
            require!(
                native_after_change < 1 || native_after_change <= native,
                MangoError::TokenInReduceOnlyMode
            );
        }
        if bank.are_borrows_reduce_only() {
            require!(
                native_after_change >= native || native_after_change >= 0,
                MangoError::TokenInReduceOnlyMode
            );
        }

        let is_active = bank.change_without_fee(
            position,
            change_amount,
            Clock::get()?.unix_timestamp.try_into().unwrap(),
        )?;
        if !is_active {
            deactivated_token_positions.push(change.raw_token_index);
        }

        if change_amount < 0 && native_after_change < 0 {
            bank.enforce_max_utilization_on_borrow()?;
            bank.check_net_borrows(*oracle_price)?;
        } else {
            bank.enforce_borrows_lte_deposits()?;
        }

        if change_amount > 0 && native_after_change > 0 {
            bank.check_deposit_and_oo_limit()?;
        }

        bank.flash_loan_approved_amount = 0;
        bank.flash_loan_token_account_initial = u64::MAX;

        token_loan_details.push(FlashLoanTokenDetailV3 {
            token_index: position.token_index,
            change_amount: change.amount.to_bits(),
            loan: loan.to_bits(),
            loan_origination_fee: loan_origination_fee.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
            price: oracle_price.to_bits(),
            swap_fee: swap_fee.to_bits(),
            approved_amount: approved_amount_u64,
        });

        emit_stack(TokenBalanceLog {
            mango_group: group.key(),
            mango_account: ctx.accounts.account.key(),
            token_index: bank.token_index as u16,
            indexed_position: position.indexed_position.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
        });
    }

    emit_stack(FlashLoanLogV3 {
        mango_group: group.key(),
        mango_account: ctx.accounts.account.key(),
        flash_loan_type,
        token_loan_details,
    });

    // Check health after account position changes
    let retriever = new_fixed_order_account_retriever(health_ais, &account.borrow())?;
    let health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)?;
    account.check_health_post(&health_cache, pre_init_health)?;

    // Deactivate inactive token accounts after health check
    for raw_token_index in deactivated_token_positions {
        account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
    }

    Ok(())
}
