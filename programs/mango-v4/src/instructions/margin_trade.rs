use crate::error::MangoError;
use crate::state::{compute_health_from_fixed_accounts, Bank, Group, HealthType, MangoAccount};
use crate::util::LoadZeroCopy;
use crate::Mango;
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;
use solana_program::instruction::Instruction;
use std::cell::Ref;
use std::collections::HashMap;

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

struct AllowedVault {
    vault_cpi_ai_index: usize,
    bank_health_ai_index: usize,
    pre_amount: u64,
}

// TODO: add loan fees
pub fn margin_trade<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, MarginTrade<'info>>,
    num_health_accounts: usize,
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
    // Note: This depends on the particular health account ordering.
    let mut allowed_banks = HashMap::<Pubkey, Ref<Bank>>::new();
    let mut allowed_vaults = HashMap::<Pubkey, usize>::new();
    let health_ais = &ctx.remaining_accounts[0..num_health_accounts];
    for (i, ai) in health_ais.iter().enumerate() {
        match ai.load::<Bank>() {
            Ok(bank) => {
                require!(bank.group == account.group, MangoError::SomeError);
                account.tokens.get_mut_or_create(bank.token_index)?;
                allowed_vaults.insert(bank.vault, i);
                allowed_banks.insert(*ai.key, bank);
            }
            Err(Error::AnchorError(error))
                if error.error_code_number == ErrorCode::AccountDiscriminatorMismatch as u32 =>
            {
                break;
            }
            Err(error) => return Err(error),
        };
    }

    let cpi_program_id = *ctx.remaining_accounts[num_health_accounts].key;
    // No self-calls via this method
    require!(
        cpi_program_id != Mango::id(),
        MangoError::InvalidMarginTradeTargetCpiProgram
    );

    // Validate the cpi accounts.
    // - Collect the signers for each used mango bank, thereby allowing
    //   withdraws from the associated vaults.
    // - Check that each group-owned token account is the vault of one of the allowed banks,
    //   and track its balance.
    let cpi_ais = &ctx.remaining_accounts[num_health_accounts + 1..];
    let mut cpi_ams = cpi_ais
        .iter()
        .flat_map(|item| item.to_account_metas(None))
        .collect::<Vec<_>>();
    require!(cpi_ais.len() == cpi_ams.len(), MangoError::SomeError);
    let mut bank_signer_data = Vec::with_capacity(allowed_banks.len());
    let mut used_vaults = Vec::with_capacity(allowed_vaults.len());
    for (i, (ai, am)) in cpi_ais.iter().zip(cpi_ams.iter_mut()).enumerate() {
        // The cpi is forbidden from calling back into mango indirectly
        require!(
            ai.key() != Mango::id(),
            MangoError::InvalidMarginTradeTargetCpiProgram
        );

        // Each allowed bank used in the cpi becomes a signer
        if ai.owner == &Mango::id() {
            if let Some(bank) = allowed_banks.get(ai.key) {
                am.is_signer = true;
                // this is the data we'll need later to build the PDA account signer seeds
                bank_signer_data.push((bank.token_index.to_le_bytes(), [bank.bump]));
            }
        }

        // Every group-owned token account must be a vault of one of the banks.
        if ai.owner == &TokenAccount::owner() {
            let token_account = Account::<TokenAccount>::try_from(ai).unwrap();
            if token_account.owner == ctx.accounts.group.key() {
                if let Some(&bank_index) = allowed_vaults.get(&ai.key) {
                    used_vaults.push(AllowedVault {
                        vault_cpi_ai_index: i,
                        bank_health_ai_index: bank_index,
                        pre_amount: token_account.amount,
                    });
                } else {
                    // This is to protect users, because if their cpi deposits to a vault and they forgot
                    // to pass in the bank for the vault, their account would not be credited.
                    require!(false, MangoError::SomeError);
                }
            }
        }
    }

    // compute pre cpi health
    let pre_cpi_health =
        compute_health_from_fixed_accounts(&account, HealthType::Maint, health_ais)?;
    require!(pre_cpi_health >= 0, MangoError::HealthMustBePositive);
    msg!("pre_cpi_health {:?}", pre_cpi_health);

    // get rid of Ref<> to avoid limiting the cpi call
    drop(allowed_banks);
    drop(group);
    drop(account);

    // prepare and invoke cpi
    let cpi_ix = Instruction {
        program_id: cpi_program_id,
        data: cpi_data,
        accounts: cpi_ams,
    };

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
    solana_program::program::invoke_signed(&cpi_ix, &cpi_ais, &signers_ref)?;

    let mut account = ctx.accounts.account.load_mut()?;

    let inactive_tokens =
        adjust_for_post_cpi_vault_amounts(health_ais, cpi_ais, used_vaults, &mut account)?;

    // compute post cpi health
    // todo: this is not working, the health is computed on old bank state and not taking into account
    // withdraws done in adjust_for_post_cpi_token_amounts
    let post_cpi_health =
        compute_health_from_fixed_accounts(&account, HealthType::Init, health_ais)?;
    require!(post_cpi_health >= 0, MangoError::HealthMustBePositive);
    msg!("post_cpi_health {:?}", post_cpi_health);

    // deactivate inactive token accounts after health check
    for raw_token_index in inactive_tokens {
        account.tokens.deactivate(raw_token_index);
    }

    Ok(())
}

fn adjust_for_post_cpi_vault_amounts(
    health_ais: &[AccountInfo],
    cpi_ais: &[AccountInfo],
    used_vaults: Vec<AllowedVault>,
    account: &mut MangoAccount,
) -> Result<Vec<usize>> {
    let mut inactive_token_raw_indexes = Vec::with_capacity(used_vaults.len());
    for info in used_vaults {
        let vault = Account::<TokenAccount>::try_from(&cpi_ais[info.vault_cpi_ai_index]).unwrap();
        let mut bank = health_ais[info.bank_health_ai_index].load_mut::<Bank>()?;
        let (position, raw_index) = account.tokens.get_mut_or_create(bank.token_index)?;
        let is_active = bank.change_with_fee(
            position,
            I80F48::from(vault.amount) - I80F48::from(info.pre_amount),
        )?;
        if !is_active {
            inactive_token_raw_indexes.push(raw_index);
        }
    }
    Ok(inactive_token_raw_indexes)
}
