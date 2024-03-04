use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::logs::{emit_stack, TokenBalanceLog, TokenForceCloseBorrowsWithTokenLogV2};
use crate::state::*;
use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub fn token_force_close_borrows_with_token(
    ctx: Context<TokenForceCloseBorrowsWithToken>,
    // which asset tokens are allowed, is checked at #3
    asset_token_index: TokenIndex,
    // token's force_close flag is checked at #2
    liab_token_index: TokenIndex,
    max_liab_transfer: u64,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();

    require_neq!(asset_token_index, liab_token_index, MangoError::SomeError);

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

    //
    // Transfer liab_token from liqor to liqee to close the borrows.
    // Transfer corresponding amount of asset_token from liqee to liqor.
    //
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();
    {
        let liqor: &mut MangoAccountRefMut = &mut liqor.borrow_mut();
        let liqor_key = ctx.accounts.liqor.key();
        let liqee: &mut MangoAccountRefMut = &mut liqee.borrow_mut();
        let liqee_key = ctx.accounts.liqee.key();

        let (asset_bank, asset_oracle_price, opt_liab_bank_and_price) =
            account_retriever.banks_mut_and_oracles(asset_token_index, liab_token_index)?;
        let (liab_bank, liab_oracle_price) = opt_liab_bank_and_price.unwrap();

        // account constraint #2
        require!(liab_bank.is_force_close(), MangoError::TokenInForceClose);

        // We might create asset borrows, so forbid asset tokens that don't allow them.
        require!(
            !asset_bank.are_borrows_reduce_only(),
            MangoError::TokenInReduceOnlyMode
        );

        let fee_factor_liqor =
            (I80F48::ONE + liab_bank.liquidation_fee) * (I80F48::ONE + asset_bank.liquidation_fee);
        let fee_factor_total =
            (I80F48::ONE + liab_bank.liquidation_fee + liab_bank.platform_liquidation_fee)
                * (I80F48::ONE + asset_bank.liquidation_fee + asset_bank.platform_liquidation_fee);

        // account constraint #3
        // only allow combination of asset and liab token,
        // where liqee's health would be guaranteed to not decrease
        require_gte!(
            liab_bank.init_liab_weight,
            asset_bank.init_liab_weight * fee_factor_total,
            MangoError::SomeError
        );

        let (liqee_asset_position, liqee_asset_raw_index) =
            liqee.token_position_and_raw_index(asset_token_index)?;
        let liqee_asset_native = liqee_asset_position.native(asset_bank);

        let (liqee_liab_position, liqee_liab_raw_index, _) =
            liqee.ensure_token_position(liab_token_index)?;
        let liqee_liab_native = liqee_liab_position.native(liab_bank);
        require!(liqee_liab_native.is_negative(), MangoError::SomeError);

        let (liqor_liab_position, liqor_liab_raw_index, _) =
            liqor.ensure_token_position(liab_token_index)?;
        let liqor_liab_native = liqor_liab_position.native(liab_bank);
        // Require that liqor obtain deposits before he tries to liquidate liqee's borrows, to prevent
        // moving other liquidator from earning further liquidation fee from these borrows by trying to liquidate the current liqor
        require!(liqor_liab_native.is_positive(), MangoError::SomeError);

        // The amount of liab native tokens we will transfer
        let max_liab_transfer = I80F48::from(max_liab_transfer);
        let liab_transfer = max_liab_transfer
            .min(-liqee_liab_native)
            .min(liqor_liab_native)
            .max(I80F48::ZERO);

        // The amount of asset native tokens we will give up for them
        let asset_transfer_base = liab_transfer * liab_oracle_price / asset_oracle_price;
        let asset_transfer_to_liqor = asset_transfer_base * fee_factor_liqor;
        let asset_transfer_from_liqee = asset_transfer_base * fee_factor_total;

        let asset_liquidation_fee = asset_transfer_from_liqee - asset_transfer_to_liqor;
        asset_bank.collected_fees_native += asset_liquidation_fee;
        asset_bank.collected_liquidation_fees += asset_liquidation_fee;

        // Apply the balance changes to the liqor and liqee accounts
        let liqee_liab_active =
            liab_bank.deposit_with_dusting(liqee_liab_position, liab_transfer, now_ts)?;
        let liqee_liab_indexed_position = liqee_liab_position.indexed_position;

        let liqor_liab_withdraw_result =
            liab_bank.withdraw_with_fee(liqor_liab_position, liab_transfer, now_ts)?;
        let liqor_liab_indexed_position = liqor_liab_position.indexed_position;
        let liqee_liab_native_after = liqee_liab_position.native(liab_bank);

        let (liqor_asset_position, liqor_asset_raw_index, _) =
            liqor.ensure_token_position(asset_token_index)?;
        let liqor_asset_active =
            asset_bank.deposit(liqor_asset_position, asset_transfer_to_liqor, now_ts)?;
        let liqor_asset_indexed_position = liqor_asset_position.indexed_position;

        let liqee_asset_position = liqee.token_position_mut_by_raw_index(liqee_asset_raw_index);
        let liqee_asset_active = asset_bank.withdraw_without_fee_with_dusting(
            liqee_asset_position,
            asset_transfer_from_liqee,
            now_ts,
        )?;
        let liqee_asset_indexed_position = liqee_asset_position.indexed_position;
        let liqee_assets_native_after = liqee_asset_position.native(asset_bank);

        msg!(
            "Force closed {} liab for {} asset",
            liab_transfer,
            asset_transfer_from_liqee,
        );

        // liqee asset
        emit_stack(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqee_key,
            token_index: asset_token_index,
            indexed_position: liqee_asset_indexed_position.to_bits(),
            deposit_index: asset_bank.deposit_index.to_bits(),
            borrow_index: asset_bank.borrow_index.to_bits(),
        });
        // liqee liab
        emit_stack(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqee_key,
            token_index: liab_token_index,
            indexed_position: liqee_liab_indexed_position.to_bits(),
            deposit_index: liab_bank.deposit_index.to_bits(),
            borrow_index: liab_bank.borrow_index.to_bits(),
        });
        // liqor asset
        emit_stack(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: asset_token_index,
            indexed_position: liqor_asset_indexed_position.to_bits(),
            deposit_index: asset_bank.deposit_index.to_bits(),
            borrow_index: asset_bank.borrow_index.to_bits(),
        });
        // liqor liab
        emit_stack(TokenBalanceLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: liab_token_index,
            indexed_position: liqor_liab_indexed_position.to_bits(),
            deposit_index: liab_bank.deposit_index.to_bits(),
            borrow_index: liab_bank.borrow_index.to_bits(),
        });

        emit_stack(TokenForceCloseBorrowsWithTokenLogV2 {
            mango_group: liqee.fixed.group,
            liqee: liqee_key,
            liqor: liqor_key,
            asset_token_index: asset_token_index,
            liab_token_index: liab_token_index,
            asset_transfer_from_liqee: asset_transfer_from_liqee.to_bits(),
            asset_transfer_to_liqor: asset_transfer_to_liqor.to_bits(),
            asset_liquidation_fee: asset_liquidation_fee.to_bits(),
            liab_transfer: liab_transfer.to_bits(),
            asset_price: asset_oracle_price.to_bits(),
            liab_price: liab_oracle_price.to_bits(),
            fee_factor: fee_factor_total.to_bits(),
        });

        // liqor should never have a borrow
        require!(
            liqor_liab_withdraw_result.loan_amount.is_zero(),
            MangoError::SomeError
        );

        let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        let liqee_health_cache = new_health_cache(&liqee.borrow(), &mut account_retriever, now_ts)
            .context("create liqee health cache")?;
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
        if !liqor_liab_withdraw_result.position_is_active {
            liqor.deactivate_token_position_and_log(liqor_liab_raw_index, liqor_key)
        }
    };

    // Check liqor's health
    // This should always improve liqor health, since we decrease the zero-asset-weight
    // liab token and gain some asset token, this check is just for denfensive measure
    let liqor_health = compute_health(
        &liqor.borrow(),
        HealthType::Init,
        &mut account_retriever,
        now_ts,
    )
    .context("compute liqor health")?;
    require!(liqor_health >= 0, MangoError::HealthMustBePositive);

    // TODO log
    // emit_stack(TokenForceCloseBorrowWithToken

    Ok(())
}
