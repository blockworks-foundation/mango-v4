use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::cmp::min;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::logs::TokenBalanceLog;
use crate::state::*;

// TODO: alternative ix name suggestions?
pub fn token_force_close_borrows_with_token(
    ctx: Context<TokenForceCloseBorrowsWithToken>,
    asset_token_index: TokenIndex,
    // token's mode is checked at #2
    liab_token_index: TokenIndex,
    max_liab_transfer: I80F48,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();

    require!(asset_token_index != liab_token_index, MangoError::SomeError);
    // TODO: should we enforce that asset token index is always USDC?

    let mut account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
        .context("create account retriever")?;

    require_keys_neq!(ctx.accounts.liqor.key(), ctx.accounts.liqee.key());
    let mut liqor = ctx.accounts.liqor.load_full_mut()?;
    // account constraint #1
    require!(
        liqor
            .fixed
            .is_owner_or_delegate(ctx.accounts.liqor_owner.key()),
        MangoError::SomeError
    );
    require_msg_typed!(
        !liqor.fixed.being_liquidated(),
        MangoError::BeingLiquidated,
        "liqor account"
    );

    let mut liqee = ctx.accounts.liqee.load_full_mut()?;
    let mut liqee_health_cache = new_health_cache(&liqee.borrow(), &account_retriever)
        .context("create liqee health cache")?;
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::Maint);

    //
    // Transfer liab_token from liqor to liqee to close the borrows.
    // Transfer corresponding amount of asset_token from liqee to liqor.
    //
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();
    {
        let account_retriever: &mut ScanningAccountRetriever = &mut account_retriever;
        let liab_token_index = liab_token_index;
        let asset_token_index = asset_token_index;
        let liqor: &mut MangoAccountRefMut = &mut liqor.borrow_mut();
        let liqor_key = ctx.accounts.liqor.key();
        let liqee: &mut MangoAccountRefMut = &mut liqee.borrow_mut();
        let liqee_key = ctx.accounts.liqee.key();

        let (asset_bank, asset_oracle_price, opt_liab_bank_and_price) =
            account_retriever.banks_mut_and_oracles(asset_token_index, liab_token_index)?;
        let (liab_bank, liab_oracle_price) = opt_liab_bank_and_price.unwrap();
        // account constraint #2
        require!(liab_bank.is_force_close(), MangoError::SomeError);

        let (liqee_asset_position, liqee_asset_raw_index) =
            liqee.token_position_and_raw_index(asset_token_index)?;
        let liqee_asset_native = liqee_asset_position.native(asset_bank);
        // TODO: should we enforce that asset position is positive? or just incur borrows if needed, I think borrowing is fine

        let (liqee_liab_position, liqee_liab_raw_index) =
            liqee.token_position_and_raw_index(liab_token_index)?;
        let liqee_liab_native = liqee_liab_position.native(liab_bank);
        require!(liqee_liab_native.is_negative(), MangoError::SomeError);

        // The amount of liab native tokens we will transfer
        let liab_transfer = min(-liqee_liab_native, max_liab_transfer);

        // The amount of asset native tokens we will give up for them
        let fee_factor = I80F48::ONE + asset_bank.liquidation_fee + liab_bank.liquidation_fee;
        let liab_oracle_price_adjusted = liab_oracle_price * fee_factor;
        let asset_transfer = liab_transfer * liab_oracle_price_adjusted / asset_oracle_price;

        // Apply the balance changes to the liqor and liqee accounts
        let liqee_liab_position = liqee.token_position_mut_by_raw_index(liqee_liab_raw_index);
        let liqee_liab_active =
            liab_bank.deposit_with_dusting(liqee_liab_position, liab_transfer, now_ts)?;
        let liqee_liab_indexed_position = liqee_liab_position.indexed_position;

        let (liqor_liab_position, liqor_liab_raw_index, _) =
            liqor.ensure_token_position(liab_token_index)?;
        let (liqor_liab_active, loan_origination_fee) = liab_bank.withdraw_with_fee(
            liqor_liab_position,
            liab_transfer,
            now_ts,
            liab_oracle_price,
        )?;
        let liqor_liab_indexed_position = liqor_liab_position.indexed_position;
        let liqee_liab_native_after = liqee_liab_position.native(liab_bank);

        let (liqor_asset_position, liqor_asset_raw_index, _) =
            liqor.ensure_token_position(asset_token_index)?;
        let liqor_asset_active =
            asset_bank.deposit(liqor_asset_position, asset_transfer, now_ts)?;
        let liqor_asset_indexed_position = liqor_asset_position.indexed_position;

        let liqee_asset_position = liqee.token_position_mut_by_raw_index(liqee_asset_raw_index);
        let liqee_asset_active = asset_bank.withdraw_without_fee_with_dusting(
            liqee_asset_position,
            asset_transfer,
            now_ts,
            asset_oracle_price,
        )?;
        let liqee_asset_indexed_position = liqee_asset_position.indexed_position;
        let liqee_assets_native_after = liqee_asset_position.native(asset_bank);

        // Update the health cache
        liqee_health_cache
            .adjust_token_balance(liab_bank, liqee_liab_native_after - liqee_liab_native)?;
        liqee_health_cache
            .adjust_token_balance(asset_bank, liqee_assets_native_after - liqee_asset_native)?;

        msg!(
            "Force closed {} liab for {} asset",
            liab_transfer,
            asset_transfer
        );

        // liqee asset
        emit!(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqee_key,
            token_index: asset_token_index,
            indexed_position: liqee_asset_indexed_position.to_bits(),
            deposit_index: asset_bank.deposit_index.to_bits(),
            borrow_index: asset_bank.borrow_index.to_bits(),
        });
        // liqee liab
        emit!(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqee_key,
            token_index: liab_token_index,
            indexed_position: liqee_liab_indexed_position.to_bits(),
            deposit_index: liab_bank.deposit_index.to_bits(),
            borrow_index: liab_bank.borrow_index.to_bits(),
        });
        // liqor asset
        emit!(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: asset_token_index,
            indexed_position: liqor_asset_indexed_position.to_bits(),
            deposit_index: asset_bank.deposit_index.to_bits(),
            borrow_index: asset_bank.borrow_index.to_bits(),
        });
        // liqor liab
        emit!(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: liab_token_index,
            indexed_position: liqor_liab_indexed_position.to_bits(),
            deposit_index: liab_bank.deposit_index.to_bits(),
            borrow_index: liab_bank.borrow_index.to_bits(),
        });

        // TODO: hmm should we just check if init_health was improved OR
        // should we use this opportunity to get the liqee out of liquidated state?
        let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
        liqee
            .fixed
            .maybe_recover_from_being_liquidated(liqee_liq_end_health);

        // Since we use a scanning account retriever, it's safe to deactivate inactive token positions
        if !liqee_asset_active {
            liqee.deactivate_token_position_and_log(liqee_asset_raw_index, liqee_key);
        }
        if !liqee_liab_active {
            liqee.deactivate_token_position_and_log(liqee_liab_raw_index, liqee_key);
        }
        if !liqor_asset_active {
            liqor.deactivate_token_position_and_log(liqor_asset_raw_index, liqor_key);
        }
        if !liqor_liab_active {
            liqor.deactivate_token_position_and_log(liqor_liab_raw_index, liqor_key)
        }
    };

    // Check liqor's health
    if !liqor.fixed.is_in_health_region() {
        let liqor_health = compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)
            .context("compute liqor health")?;
        require!(liqor_health >= 0, MangoError::HealthMustBePositive);
    }

    // TODO log
    // emit!(TokenForceCloseBorrowWithToken

    Ok(())
}
