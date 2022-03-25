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
    let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts)?;
    // TODO: this 0 argument is awkward
    // TODO: get mut data here
    let (asset_bank, asset_oracle) =
        account_retriever.bank_and_oracle(group_pk, 0, asset_token_index)?;
    let (liab_bank, liab_oracle) =
        account_retriever.bank_and_oracle(group_pk, 0, liab_token_index)?;
    let asset_price = oracle_price(asset_oracle)?;
    let liab_price = oracle_price(liab_oracle)?;

    //
    // Health computation
    //
    let liqee = ctx.accounts.liqee.load()?;
    // TODO: reuse the account_retriever
    let init_health = compute_health_by_scanning_accounts(&liqee, ctx.remaining_accounts)?;
    msg!("health: {}", init_health);
    // TODO: actual check involving being_liquidated and maint_health
    require!(init_health < 0, MangoError::SomeError);

    // TODO: Should we fail if liqee still has open spot orders? mango-v3 is ok with that.

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

    // TODO: get/compute these
    let (liab_fee, init_liab_weight) = (I80F48::ONE, I80F48::ONE);
    let (asset_fee, init_asset_weight) = (I80F48::ONE, I80F48::ONE);

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
    let liab_needed =
        cm!(-init_health
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

    // TODO: apply to liqee and liqor accounts
    // to do this we need mut Banks from remainingAccounts

    // TODO: Check liqor's health
    // TODO: Check liqee's health again

    Ok(())
}
