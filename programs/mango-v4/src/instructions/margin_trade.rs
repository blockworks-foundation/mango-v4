use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::{compute_health_from_fixed_accounts, Bank, Group, HealthType, MangoAccount};
use crate::{group_seeds, Mango};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use fixed::types::I80F48;
use solana_program::instruction::Instruction;
use std::cell::Ref;
use std::collections::HashMap;

/// The margin trade instruction
///
/// In addition to these accounts, there must be a sequence of remaining_accounts:
/// 1. health_accounts: accounts needed for health checking
/// 2. target_program_id: the target program account
/// 3. target_accounts: the accounts to pass to the target program
///
/// Every vault address listed in 3. must also have the matching bank and oracle appear in 1.
///
/// Every vault that is to be withdrawn from must appear in the `withdraws` instruction argument.
/// The corresponding bank may be used as an authority for vault withdrawals.
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
    pub token_program: Program<'info, Token>,
}

struct AllowedVault {
    /// index of the vault in cpi_ais
    vault_cpi_ai_index: usize,
    /// index of the bank in health_ais
    bank_health_ai_index: usize,
    /// raw index into account.tokens
    raw_token_index: usize,
    /// vault amount before cpi
    pre_amount: u64,
    /// requested withdraw amount
    withdraw_amount: u64,
    /// amount of withdraw request that is a loan
    loan_amount: I80F48,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy)]
pub struct MarginTradeWithdraw {
    /// Account index of the vault to withdraw from in the target_accounts section.
    /// Meaning that the first account after target_program_id would have index 0.
    pub index: u8,
    /// Requested withdraw amount.
    pub amount: u64,
}

/// - `num_health_accounts` is the number of health accounts that remaining_accounts starts with.
/// - `withdraws` is a list of MarginTradeWithdraw requests.
/// - `cpi_data` is the bytes to call the target_program_id with.
pub fn margin_trade<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, MarginTrade<'info>>,
    num_health_accounts: usize,
    withdraws: Vec<MarginTradeWithdraw>,
    cpi_data: Vec<u8>,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let mut account = ctx.accounts.account.load_mut()?;
    require!(account.is_bankrupt == 0, MangoError::IsBankrupt);

    // Go over the banks passed as health accounts and:
    // - Ensure that all banks that are passed in have activated positions.
    //   This is necessary because maybe the user wants to margin trade on a token
    //   that the account hasn't used before.
    // - Collect the addresses of all banks to potentially sign for in cpi_ais.
    // - Collect the addresses of all bank vaults.
    // Note: This depends on the particular health account ordering.
    let health_ais = &ctx.remaining_accounts[0..num_health_accounts];
    let mut allowed_banks = HashMap::<&Pubkey, Ref<Bank>>::new();
    // vault pubkey -> (bank_account_index, raw_token_index)
    let mut allowed_vaults = HashMap::<Pubkey, (usize, usize)>::new();
    for (i, ai) in health_ais.iter().enumerate() {
        match ai.load::<Bank>() {
            Ok(bank) => {
                require!(bank.group == account.group, MangoError::SomeError);
                let (_, raw_token_index) = account.tokens.get_mut_or_create(bank.token_index)?;
                allowed_vaults.insert(bank.vault, (i, raw_token_index));
                allowed_banks.insert(ai.key, bank);
            }
            Err(Error::AnchorError(error))
                if error.error_code_number == ErrorCode::AccountDiscriminatorMismatch as u32
                    || error.error_code_number == ErrorCode::AccountOwnedByWrongProgram as u32 =>
            {
                break;
            }
            Err(error) => return Err(error),
        };
    }

    // Check pre-cpi health
    // NOTE: This health check isn't strictly necessary. It will be, later, when
    // we want to have reduce_only or be able to move an account out of bankruptcy.
    let pre_cpi_health =
        compute_health_from_fixed_accounts(&account, HealthType::Init, health_ais)?;
    require!(pre_cpi_health >= 0, MangoError::HealthMustBePositive);
    msg!("pre_cpi_health {:?}", pre_cpi_health);

    let cpi_program_id = *ctx.remaining_accounts[num_health_accounts].key;
    require_keys_neq!(cpi_program_id, crate::id(), MangoError::SomeError);

    let cpi_ais = &ctx.remaining_accounts[num_health_accounts + 1..];
    let mut cpi_ams = cpi_ais
        .iter()
        .flat_map(|item| item.to_account_metas(None))
        .collect::<Vec<_>>();
    require!(cpi_ais.len() == cpi_ams.len(), MangoError::SomeError);

    // Check that each group-owned token account is the vault of one of the allowed banks,
    // and track its balance.
    let mut used_vaults = cpi_ais
        .iter()
        .enumerate()
        .filter_map(|(i, ai)| {
            if ai.owner != &TokenAccount::owner() {
                return None;
            }

            // Skip mints and other accounts that may be owned by the spl_token program
            let maybe_token_account = Account::<TokenAccount>::try_from(ai);
            if maybe_token_account.is_err() {
                return None;
            }

            let token_account = maybe_token_account.unwrap();
            if token_account.owner != ctx.accounts.group.key() {
                return None;
            }

            // Every group-owned token account must be a vault of one of the banks.
            if let Some(&(bank_index, raw_token_index)) = allowed_vaults.get(&ai.key) {
                return Some(Ok((
                    ai.key,
                    AllowedVault {
                        vault_cpi_ai_index: i,
                        bank_health_ai_index: bank_index,
                        raw_token_index,
                        pre_amount: token_account.amount,
                        // these two are updated later
                        withdraw_amount: 0,
                        loan_amount: I80F48::ZERO,
                    },
                )));
            }

            // This is to protect users, because if their cpi program sends deposits to a vault
            // and they forgot to pass in the bank for the vault, their account would not be credited.
            Some(Err(error!(MangoError::SomeError)))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    // Find banks for used vaults in cpi_ais and collect signer seeds for them.
    // Also update withdraw_amount and loan_amount.
    let mut bank_signer_data = Vec::with_capacity(used_vaults.len());
    for (ai, am) in cpi_ais.iter().zip(cpi_ams.iter_mut()) {
        if ai.owner != &Mango::id() {
            continue;
        }
        if let Some(bank) = allowed_banks.get(ai.key) {
            if let Some(vault_info) = used_vaults.get_mut(&bank.vault) {
                let withdraw_amount = withdraws
                    .iter()
                    .find_map(|&withdraw| {
                        (withdraw.index as usize == vault_info.vault_cpi_ai_index)
                            .then(|| withdraw.amount)
                    })
                    // Even if we don't withdraw from a vault we still need to track it:
                    // Possibly the invoked program will deposit funds into it.
                    .unwrap_or(0);
                require!(
                    withdraw_amount <= vault_info.pre_amount,
                    MangoError::SomeError
                );
                vault_info.withdraw_amount = withdraw_amount;

                // if there are withdraws: figure out loan amount, mark as signer
                if withdraw_amount > 0 {
                    let token_account = account.tokens.get_mut_raw(vault_info.raw_token_index);
                    let native_position = token_account.native(&bank);
                    vault_info.loan_amount = if native_position > 0 {
                        (I80F48::from(withdraw_amount) - native_position).max(I80F48::ZERO)
                    } else {
                        I80F48::from(withdraw_amount)
                    };

                    am.is_signer = true;
                    // this is the data we'll need later to build the PDA account signer seeds
                    bank_signer_data.push((bank.token_index.to_le_bytes(), [bank.bump]));
                }
            }
        }
    }

    // Approve bank delegates for withdrawals
    let group_seeds = group_seeds!(group);
    let seeds = [&group_seeds[..]];
    for (_, vault_info) in used_vaults.iter() {
        if vault_info.withdraw_amount > 0 {
            let approve_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Approve {
                    to: cpi_ais[vault_info.vault_cpi_ai_index].clone(),
                    delegate: health_ais[vault_info.bank_health_ai_index].clone(),
                    authority: ctx.accounts.group.to_account_info(),
                },
            )
            .with_signer(&seeds);
            token::approve(approve_ctx, vault_info.withdraw_amount)?;
        }
    }

    // get rid of Ref<> to avoid limiting the cpi call
    drop(allowed_banks);
    drop(group);
    drop(account);

    // prepare signer seeds and invoke cpi
    let group_key = ctx.accounts.group.key();
    let signers = bank_signer_data
        .iter()
        .map(|(token_index, bump)| {
            [
                group_key.as_ref(),
                b"Bank".as_ref(),
                &token_index[..],
                &bump[..],
            ]
        })
        .collect::<Vec<_>>();
    let signers_ref = signers.iter().map(|v| &v[..]).collect::<Vec<_>>();
    let cpi_ix = Instruction {
        program_id: cpi_program_id,
        data: cpi_data,
        accounts: cpi_ams,
    };
    solana_program::program::invoke_signed(&cpi_ix, &cpi_ais, &signers_ref)?;

    // Revoke delegates for vaults
    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);
    for (_, vault_info) in used_vaults.iter() {
        if vault_info.withdraw_amount > 0 {
            let ix = token::spl_token::instruction::revoke(
                &token::spl_token::ID,
                &cpi_ais[vault_info.vault_cpi_ai_index].key,
                &ctx.accounts.group.key(),
                &[],
            )?;
            solana_program::program::invoke_signed(
                &ix,
                &[
                    cpi_ais[vault_info.vault_cpi_ai_index].clone(),
                    ctx.accounts.group.to_account_info(),
                ],
                &[group_seeds],
            )?;
        }
    }

    // Track vault changes and apply them to the user's token positions
    let mut account = ctx.accounts.account.load_mut()?;
    let inactive_tokens =
        adjust_for_post_cpi_vault_amounts(health_ais, cpi_ais, &used_vaults, &mut account)?;

    // Check post-cpi health
    let post_cpi_health =
        compute_health_from_fixed_accounts(&account, HealthType::Init, health_ais)?;
    require!(post_cpi_health >= 0, MangoError::HealthMustBePositive);
    msg!("post_cpi_health {:?}", post_cpi_health);

    // Deactivate inactive token accounts after health check
    for raw_token_index in inactive_tokens {
        account.tokens.deactivate(raw_token_index);
    }

    Ok(())
}

fn adjust_for_post_cpi_vault_amounts(
    health_ais: &[AccountInfo],
    cpi_ais: &[AccountInfo],
    used_vaults: &HashMap<&Pubkey, AllowedVault>,
    account: &mut MangoAccount,
) -> Result<Vec<usize>> {
    let mut inactive_token_raw_indexes = Vec::with_capacity(used_vaults.len());
    for (_, info) in used_vaults.iter() {
        let vault = Account::<TokenAccount>::try_from(&cpi_ais[info.vault_cpi_ai_index]).unwrap();
        let mut bank = health_ais[info.bank_health_ai_index].load_mut::<Bank>()?;
        let position = account.tokens.get_mut_raw(info.raw_token_index);

        let loan_origination_fee = info.loan_amount * bank.loan_origination_fee_rate;
        bank.collected_fees_native += loan_origination_fee;

        let is_active = bank.change_without_fee(
            position,
            I80F48::from(vault.amount) - I80F48::from(info.pre_amount) - loan_origination_fee,
        )?;
        if !is_active {
            inactive_token_raw_indexes.push(info.raw_token_index);
        }
    }
    Ok(inactive_token_raw_indexes)
}
