use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::cmp::min;

use crate::error::*;
use crate::state::*;
use crate::state::{oracle_price, AccountRetriever, ScanningAccountRetriever};
use crate::util::checked_math as cm;

#[derive(Accounts)]
pub struct LiqTokenWithToken<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = liqor.load()?.owner == liqor_owner.key(),
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
    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)?;

    let mut liqor = ctx.accounts.liqor.load_mut()?;
    let mut liqee = ctx.accounts.liqee.load_mut()?;

    //
    // Health computation
    //
    let init_health = compute_health(&liqee, &account_retriever)?;
    msg!("health: {}", init_health);
    // TODO: actual check involving being_liquidated and maint_health
    require!(init_health < 0, MangoError::SomeError);

    // TODO: Should we fail if liqee still has open spot orders? mango-v3 is ok with that.

    //
    // Transfer some liab_token from liqor to liqee and
    // transfer some asset_token from liqee to liqor.
    //
    {
        // Get the mut banks and oracle prices
        //
        // This must happen _after_ the health computation, since immutable borrows of
        // the bank are not allowed at the same time.
        let (mut asset_bank, asset_oracle) =
            account_retriever.bank_mut_and_oracle(asset_token_index)?;
        let (mut liab_bank, liab_oracle) =
            account_retriever.bank_mut_and_oracle(liab_token_index)?;
        let asset_price = oracle_price(asset_oracle)?;
        let liab_price = oracle_price(liab_oracle)?;

        let liqee_assets_native = liqee
            .token_account_map
            .get(asset_bank.token_index)?
            .native(&asset_bank);
        require!(liqee_assets_native.is_positive(), MangoError::SomeError);

        let liqee_liab_native = liqee
            .token_account_map
            .get(liab_bank.token_index)?
            .native(&liab_bank);
        require!(liqee_liab_native.is_negative(), MangoError::SomeError);

        // these factors get divided into the price, making asset worth less
        // and liab worth more
        let asset_fee = I80F48::ONE + asset_bank.liquidation_fee;
        let liab_fee = I80F48::ONE - liab_bank.liquidation_fee;

        let init_asset_weight = asset_bank.init_asset_weight;
        let init_liab_weight = liab_bank.init_liab_weight;

        // How much asset would need to be exchanged to liab in order to bring health to 0?
        //
        // That means: what is x (unit: native liab tokens) such that
        //   init_health + x * ilw * lp - y * iaw * ap = 0
        // where
        //   ilw = init_liab_weight, lp = liab_price, lf = liab_fee
        //   iap = init_asset_weight, ap = asset_price, af = asset_fee
        // and the asset cost of getting x native units of liab is:
        //   y = x * (lp / lf) / (ap / af)      (native asset tokens)
        //
        // Result: x = -init_health / (lp * (ilw - iaw * af / lf))
        let liab_needed = cm!(-init_health
            / (liab_price * (init_liab_weight - init_asset_weight * asset_fee / liab_fee)));

        // How much liab can we get at most for the asset balance?
        let liab_possible =
            cm!(liqee_assets_native * asset_price * liab_fee / (liab_price * asset_fee));

        // The amount of liab native tokens we will transfer
        let liab_transfer = min(
            min(min(liab_needed, -liqee_liab_native), liab_possible),
            max_liab_transfer,
        );

        // The amount of asset native tokens we will give up for them
        let asset_transfer = cm!(liab_transfer * asset_price * liab_fee / (liab_price * asset_fee));

        // Apply the balance changes to the liqor and liqee accounts
        liab_bank.deposit(
            liqee.token_account_map.get_mut(liab_token_index)?,
            liab_transfer,
        )?;
        liab_bank.withdraw(
            liqor
                .token_account_map
                .get_mut_or_create(liab_token_index)?
                .0,
            liab_transfer,
        )?;

        asset_bank.deposit(
            liqor
                .token_account_map
                .get_mut_or_create(asset_token_index)?
                .0,
            asset_transfer,
        )?;
        asset_bank.withdraw(
            liqee.token_account_map.get_mut(asset_token_index)?,
            asset_transfer,
        )?;
    }

    // TODO: Check liqor's health
    // TODO: Check liqee's health again

    // TOOD: this must deactivate token accounts if the deposit/withdraw calls above call for it

    Ok(())
}
