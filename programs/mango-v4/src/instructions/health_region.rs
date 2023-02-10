use crate::error::*;
use crate::health::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_lang::Discriminator;
use fixed::types::I80F48;

/// Sets up for a health region
///
/// The same transaction must have the corresponding HealthRegionEnd call.
///
/// remaining_accounts: health accounts for account
#[derive(Accounts)]
pub struct HealthRegionBegin<'info> {
    /// Instructions Sysvar for instruction introspection
    /// CHECK: fixed instructions sysvar account
    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::HealthRegion) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}

/// Ends a health region.
///
/// remaining_accounts: health accounts for account
#[derive(Accounts)]
pub struct HealthRegionEnd<'info> {
    #[account(
        mut,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
}

pub fn health_region_begin<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionBegin<'info>>,
) -> Result<()> {
    // Check if the other instructions in the transactions are compatible
    {
        let ixs = ctx.accounts.instructions.as_ref();
        let current_index = tx_instructions::load_current_index_checked(ixs)? as usize;

        // There must be a matching HealthRegionEnd instruction
        let mut index = current_index + 1;
        let mut found_end = false;
        loop {
            let ix = match tx_instructions::load_instruction_at_checked(index, ixs) {
                Ok(ix) => ix,
                Err(ProgramError::InvalidArgument) => break, // past the last instruction
                Err(e) => return Err(e.into()),
            };
            index += 1;

            if ix.program_id != crate::id() {
                continue;
            }
            if ix.data[0..8] != crate::instruction::HealthRegionEnd::discriminator() {
                continue;
            }

            // check that it's for the same account
            require_keys_eq!(ix.accounts[0].pubkey, ctx.accounts.account.key());

            found_end = true;
            index += 1;
        }
        require_msg!(
            found_end,
            "found no HealthRegionEnd instruction in transaction"
        );
    }

    let mut account = ctx.accounts.account.load_full_mut()?;
    require_msg!(
        !account.fixed.is_in_health_region(),
        "account must not already be health wrapped"
    );
    account.fixed.set_in_health_region(true);

    let group = account.fixed.group;
    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &group)
        .context("create account retriever")?;

    // Compute pre-health and store it on the account
    let health_cache = new_health_cache(&account.borrow(), &account_retriever)?;
    let pre_init_health = account.check_health_pre(&health_cache)?;
    account.fixed.health_region_begin_init_health =
        pre_init_health.ceil().checked_to_num().unwrap();

    Ok(())
}

pub fn health_region_end<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionEnd<'info>>,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    require_msg!(
        account.fixed.is_in_health_region(),
        "account must be health wrapped"
    );
    account.fixed.set_in_health_region(false);

    let group = account.fixed.group;
    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &group)
        .context("create account retriever")?;
    let health_cache = new_health_cache(&account.borrow(), &account_retriever)?;

    let pre_init_health = I80F48::from(account.fixed.health_region_begin_init_health);
    account.check_health_post(&health_cache, pre_init_health)?;
    account.fixed.health_region_begin_init_health = 0;

    Ok(())
}
