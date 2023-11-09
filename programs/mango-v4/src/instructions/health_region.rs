use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_lang::Discriminator;
use fixed::types::I80F48;

pub fn health_region_begin<'key, 'accounts, 'remaining, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, HealthRegionBegin<'info>>,
) -> Result<()> {
    // The instructions that may be called inside a HealthRegion
    let allowed_inner_ix = [
        crate::instruction::PerpCancelAllOrders::discriminator(),
        crate::instruction::PerpCancelAllOrdersBySide::discriminator(),
        crate::instruction::PerpCancelOrder::discriminator(),
        crate::instruction::PerpCancelOrderByClientOrderId::discriminator(),
        crate::instruction::PerpPlaceOrder::discriminator(),
        crate::instruction::PerpPlaceOrderV2::discriminator(),
        crate::instruction::PerpPlaceOrderPegged::discriminator(),
        crate::instruction::PerpPlaceOrderPeggedV2::discriminator(),
        crate::instruction::Serum3CancelAllOrders::discriminator(),
        crate::instruction::Serum3CancelOrder::discriminator(),
        crate::instruction::Serum3PlaceOrder::discriminator(),
        crate::instruction::Serum3PlaceOrderV2::discriminator(),
        crate::instruction::Serum3SettleFunds::discriminator(),
        crate::instruction::Serum3SettleFundsV2::discriminator(),
    ];

    // Check if the other instructions in the transaction are compatible
    {
        let ixs = ctx.accounts.instructions.as_ref();
        let current_index = tx_instructions::load_current_index_checked(ixs)? as usize;

        // Forbid HealthRegionBegin to be called from CPI (it does not have to be the first instruction)
        let current_ix = tx_instructions::load_instruction_at_checked(current_index, ixs)?;
        require_msg!(
            current_ix.program_id == *ctx.program_id,
            "HealthRegionBegin must be a top-level instruction"
        );

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

            require_keys_eq!(
                ix.program_id,
                crate::id(),
                MangoError::HealthRegionBadInnerInstruction
            );

            let discriminator: [u8; 8] = ix.data[0..8].try_into().unwrap();
            if discriminator == crate::instruction::HealthRegionEnd::discriminator() {
                // check that it's for the same account
                require_keys_eq!(ix.accounts[0].pubkey, ctx.accounts.account.key());
                found_end = true;
                break;
            } else {
                require!(
                    allowed_inner_ix.contains(&discriminator),
                    MangoError::HealthRegionBadInnerInstruction
                );
            }
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
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let health_cache = new_health_cache(&account.borrow(), &account_retriever, now_ts)?;
    let pre_init_health = account.check_health_pre(&health_cache)?;
    account.fixed.health_region_begin_init_health = pre_init_health.ceil().to_num();

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
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let health_cache = new_health_cache(&account.borrow(), &account_retriever, now_ts)?;

    let pre_init_health = I80F48::from(account.fixed.health_region_begin_init_health);
    account.check_health_post(&health_cache, pre_init_health)?;
    account.fixed.health_region_begin_init_health = 0;

    Ok(())
}
