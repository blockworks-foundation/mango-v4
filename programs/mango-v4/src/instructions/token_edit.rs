use anchor_lang::prelude::*;

use fixed::types::I80F48;

use super::InterestRateParams;
use crate::error::MangoError;
use crate::state::*;

#[derive(Accounts)]
#[instruction(token_index: TokenIndex, bank_num: u64)]
pub struct TokenEdit<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"Bank".as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        has_one = group
    )]
    pub bank: AccountLoader<'info, Bank>,
}

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn token_edit(
    ctx: Context<TokenEdit>,
    token_index: TokenIndex,
    bank_num: u64,
    oracle_opt: Option<Pubkey>,
    oracle_config_opt: Option<OracleConfig>,
    interest_rate_params_opt: Option<InterestRateParams>,
    loan_fee_rate_opt: Option<f32>,
    loan_origination_fee_rate_opt: Option<f32>,
    maint_asset_weight_opt: Option<f32>,
    init_asset_weight_opt: Option<f32>,
    maint_liab_weight_opt: Option<f32>,
    init_liab_weight_opt: Option<f32>,
    liquidation_fee_opt: Option<f32>,
) -> Result<()> {
    require!(
        oracle_opt.is_some()
            || interest_rate_params_opt.is_some()
            || loan_origination_fee_rate_opt.is_some()
            || maint_asset_weight_opt.is_some(),
        MangoError::SomeError
    );

    let mut bank = ctx.accounts.bank.load_mut()?;

    msg!("old bank {:#?}", bank);

    // note: unchanged fields are inline, and match exact definition in register_token
    // please maintain, and don't remove, makes it easy to reason about which support admin modification

    // unchanged -
    // name
    // group
    // mint
    // vault

    if let Some(oracle) = oracle_opt {
        bank.oracle = oracle;
        bank.oracle_config = oracle_config_opt.unwrap();
    };

    // unchanged -
    // deposit_index
    // borrow_index
    // cached_indexed_total_deposits
    // cached_indexed_total_borrows
    // indexed_deposits
    // indexed_borrows
    // last_updated

    if let Some(interest_rate_params) = interest_rate_params_opt {
        // TODO: add a require! verifying relation between the parameters
        bank.util0 = I80F48::from_num(interest_rate_params.util0);
        bank.rate0 = I80F48::from_num(interest_rate_params.rate0);
        bank.util1 = I80F48::from_num(interest_rate_params.util1);
        bank.rate1 = I80F48::from_num(interest_rate_params.rate1);
        bank.max_rate = I80F48::from_num(interest_rate_params.max_rate);
    }

    // unchanged -
    // collected_fees_native

    if let Some(loan_origination_fee_rate) = loan_origination_fee_rate_opt {
        bank.loan_origination_fee_rate = I80F48::from_num(loan_origination_fee_rate);
        bank.loan_fee_rate = I80F48::from_num(loan_fee_rate_opt.unwrap());
    }

    if let Some(maint_asset_weight) = maint_asset_weight_opt {
        bank.maint_asset_weight = I80F48::from_num(maint_asset_weight);
        bank.init_asset_weight = I80F48::from_num(init_asset_weight_opt.unwrap());
        bank.maint_liab_weight = I80F48::from_num(maint_liab_weight_opt.unwrap());
        bank.init_liab_weight = I80F48::from_num(init_liab_weight_opt.unwrap());
        bank.liquidation_fee = I80F48::from_num(liquidation_fee_opt.unwrap());
    }

    // unchanged -
    // dust
    // flash_loan_vault_initial
    // flash_loan_approved_amount
    // token_index
    // bump
    // mint_decimals
    // bank_num
    // reserved

    msg!("new bank {:#?}", bank);

    Ok(())
}
