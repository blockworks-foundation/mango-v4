use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::cmp::min;

use crate::error::*;
use crate::logs::{LiquidateTokenAndTokenLog, TokenBalanceLog};
use crate::state::ScanningAccountRetriever;
use crate::state::*;
use crate::util::checked_math as cm;

#[derive(Accounts)]
pub struct LiqTokenWithToken<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.is_owner_or_delegate(liqor_owner.key()),
    )]
    pub liqor: AccountLoader<'info, MangoAccount>,
    pub liqor_owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
    )]
    pub liqee: AccountLoader<'info, MangoAccount>,
}

pub fn liq_token_with_token(
    ctx: Context<LiqTokenWithToken>,
    asset_token_index: TokenIndex,
    liab_token_index: TokenIndex,
    max_liab_transfer: I80F48,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();

    require!(asset_token_index != liab_token_index, MangoError::SomeError);
    let mut account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)?;

    let mut liqor = ctx.accounts.liqor.load_mut()?;
    require!(!liqor.is_bankrupt(), MangoError::IsBankrupt);

    let mut liqee = ctx.accounts.liqee.load_mut()?;
    require!(!liqee.is_bankrupt(), MangoError::IsBankrupt);

    // Initial liqee health check
    let mut liqee_health_cache = new_health_cache(&liqee, &account_retriever)?;
    let init_health = liqee_health_cache.health(HealthType::Init)?;
    if liqee.being_liquidated() {
        if init_health > I80F48::ZERO {
            liqee.set_being_liquidated(false);
            msg!("Liqee init_health above zero");
            return Ok(());
        }
    } else {
        let maint_health = liqee_health_cache.health(HealthType::Maint)?;
        require!(maint_health < I80F48::ZERO, MangoError::SomeError);
        liqee.set_being_liquidated(true);
    }

    //
    // Transfer some liab_token from liqor to liqee and
    // transfer some asset_token from liqee to liqor.
    //
    {
        // Get the mut banks and oracle prices
        //
        // This must happen _after_ the health computation, since immutable borrows of
        // the bank are not allowed at the same time.
        let (asset_bank, asset_price, opt_liab_bank_and_price) =
            account_retriever.banks_mut_and_oracles(asset_token_index, liab_token_index)?;
        let (liab_bank, liab_price) = opt_liab_bank_and_price.unwrap();

        // The main complication here is that we can't keep the liqee_asset_position and liqee_liab_position
        // borrows alive at the same time. Possibly adding get_mut_pair() would be helpful.
        let (liqee_asset_position, liqee_asset_raw_index) =
            liqee.tokens.get_mut(asset_token_index)?;
        let liqee_assets_native = liqee_asset_position.native(&asset_bank);
        require!(liqee_assets_native.is_positive(), MangoError::SomeError);

        let (liqee_liab_position, liqee_liab_raw_index) = liqee.tokens.get_mut(liab_token_index)?;
        let liqee_liab_native = liqee_liab_position.native(&liab_bank);
        require!(liqee_liab_native.is_negative(), MangoError::SomeError);

        // TODO why sum of both tokens liquidation fees? Add comment
        let fee_factor = I80F48::ONE + asset_bank.liquidation_fee + liab_bank.liquidation_fee;
        let liab_price_adjusted = liab_price * fee_factor;

        let init_asset_weight = asset_bank.init_asset_weight;
        let init_liab_weight = liab_bank.init_liab_weight;

        // How much asset would need to be exchanged to liab in order to bring health to 0?
        //
        // That means: what is x (unit: native liab tokens) such that
        //   init_health + x * ilw * lp - y * iaw * ap = 0
        // where
        //   ilw = init_liab_weight, lp = liab_price
        //   iap = init_asset_weight, ap = asset_price
        //   ff = fee_factor, lpa = lp * ff
        // and the asset cost of getting x native units of liab is:
        //   y = x * lp / ap * ff = x * lpa / ap   (native asset tokens)
        //
        // Result: x = -init_health / (lp * ilw - iaw * lpa)
        let liab_needed = cm!(-init_health
            / (liab_price * init_liab_weight - init_asset_weight * liab_price_adjusted));

        // How much liab can we get at most for the asset balance?
        let liab_possible = cm!(liqee_assets_native * asset_price / liab_price_adjusted);

        // The amount of liab native tokens we will transfer
        let liab_transfer = min(
            min(min(liab_needed, -liqee_liab_native), liab_possible),
            max_liab_transfer,
        );

        // The amount of asset native tokens we will give up for them
        let asset_transfer = cm!(liab_transfer * liab_price_adjusted / asset_price);

        // Apply the balance changes to the liqor and liqee accounts
        let liqee_liab_position = liqee.tokens.get_mut_raw(liqee_liab_raw_index);
        let liqee_liab_active = liab_bank.deposit(liqee_liab_position, liab_transfer)?;
        let liqee_liab_position_indexed = liqee_liab_position.indexed_position;

        let (liqor_liab_position, liqor_liab_raw_index, _) =
            liqor.tokens.get_mut_or_create(liab_token_index)?;
        let liqor_liab_active = liab_bank.withdraw_with_fee(liqor_liab_position, liab_transfer)?;
        let liqor_liab_position_indexed = liqor_liab_position.indexed_position;

        let (liqor_asset_position, liqor_asset_raw_index, _) =
            liqor.tokens.get_mut_or_create(asset_token_index)?;
        let liqor_asset_active = asset_bank.deposit(liqor_asset_position, asset_transfer)?;
        let liqor_asset_position_indexed = liqor_asset_position.indexed_position;

        let liqee_asset_position = liqee.tokens.get_mut_raw(liqee_asset_raw_index);
        let liqee_asset_active =
            asset_bank.withdraw_without_fee(liqee_asset_position, asset_transfer)?;
        let liqee_asset_position_indexed = liqee_asset_position.indexed_position;

        // Update the health cache
        liqee_health_cache.adjust_token_balance(liab_token_index, liab_transfer)?;
        liqee_health_cache.adjust_token_balance(asset_token_index, -asset_transfer)?;

        msg!(
            "liquidated {} liab for {} asset",
            liab_transfer,
            asset_transfer
        );

        emit!(LiquidateTokenAndTokenLog {
            liqee: ctx.accounts.liqee.key(),
            liqor: ctx.accounts.liqor.key(),
            asset_token_index: asset_token_index,
            liab_token_index: liab_token_index,
            asset_transfer: asset_transfer.to_bits(),
            liab_transfer: liab_transfer.to_bits(),
            asset_price: asset_price.to_bits(),
            liab_price: liab_price.to_bits(),
            // bankruptcy:
        });

        // liqee asset
        emit!(TokenBalanceLog {
            mango_account: ctx.accounts.liqee.key(),
            token_index: asset_token_index,
            indexed_position: liqee_asset_position_indexed.to_bits(),
            deposit_index: asset_bank.deposit_index.to_bits(),
            borrow_index: asset_bank.borrow_index.to_bits(),
            price: asset_price.to_bits(),
        });
        // liqee liab
        emit!(TokenBalanceLog {
            mango_account: ctx.accounts.liqee.key(),
            token_index: liab_token_index,
            indexed_position: liqee_liab_position_indexed.to_bits(),
            deposit_index: liab_bank.deposit_index.to_bits(),
            borrow_index: liab_bank.borrow_index.to_bits(),
            price: liab_price.to_bits(),
        });
        // liqor asset
        emit!(TokenBalanceLog {
            mango_account: ctx.accounts.liqor.key(),
            token_index: asset_token_index,
            indexed_position: liqor_asset_position_indexed.to_bits(),
            deposit_index: asset_bank.deposit_index.to_bits(),
            borrow_index: asset_bank.borrow_index.to_bits(),
            price: asset_price.to_bits(),
        });
        // liqor liab
        emit!(TokenBalanceLog {
            mango_account: ctx.accounts.liqor.key(),
            token_index: liab_token_index,
            indexed_position: liqor_liab_position_indexed.to_bits(),
            deposit_index: liab_bank.deposit_index.to_bits(),
            borrow_index: liab_bank.borrow_index.to_bits(),
            price: liab_price.to_bits(),
        });

        // Since we use a scanning account retriever, it's safe to deactivate inactive token positions
        if !liqee_asset_active {
            liqee.tokens.deactivate(liqee_asset_raw_index);
        }
        if !liqee_liab_active {
            liqee.tokens.deactivate(liqee_liab_raw_index);
        }
        if !liqor_asset_active {
            liqor.tokens.deactivate(liqor_asset_raw_index);
        }
        if !liqor_liab_active {
            liqor.tokens.deactivate(liqor_liab_raw_index)
        }
    }

    // Check liqee health again
    let maint_health = liqee_health_cache.health(HealthType::Maint)?;
    if maint_health < I80F48::ZERO {
        liqee.set_bankrupt(!liqee_health_cache.has_liquidatable_assets());
    } else {
        let init_health = liqee_health_cache.health(HealthType::Init)?;

        // this is equivalent to one native USDC or 1e-6 USDC
        // This is used as threshold to flip flag instead of 0 because of dust issues
        liqee.set_being_liquidated(init_health < -I80F48::ONE);
    }

    // Check liqor's health
    let liqor_health = compute_health(&liqor, HealthType::Init, &account_retriever)?;
    require!(liqor_health >= 0, MangoError::HealthMustBePositive);

    Ok(())
}
