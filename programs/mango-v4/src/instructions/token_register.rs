use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fixed::types::I80F48;
use fixed_macro::types::I80F48;

use crate::error::*;
use crate::state::*;
use crate::util::fill16_from_str;

pub const INDEX_START: I80F48 = I80F48!(1_000_000);

#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct TokenRegister<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [group.key().as_ref(), b"Bank".as_ref(), &token_index.to_le_bytes(), &0u32.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"Vault".as_ref(), &token_index.to_le_bytes(), &0u32.to_le_bytes()],
        bump,
        token::authority = group,
        token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init,
        // using the mint in this seed guards against registering the same mint twice
        seeds = [group.key().as_ref(), b"MintInfo".as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MintInfo>(),
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InterestRateParams {
    pub util0: f32,
    pub rate0: f32,
    pub util1: f32,
    pub rate1: f32,
    pub max_rate: f32,
    pub adjustment_factor: f32,
}

// TODO: should this be "configure_mint", we pass an explicit index, and allow
// overwriting config as long as the mint account stays the same?
#[allow(clippy::too_many_arguments)]
pub fn token_register(
    ctx: Context<TokenRegister>,
    token_index: TokenIndex,
    name: String,
    oracle_config: OracleConfig,
    interest_rate_params: InterestRateParams,
    loan_fee_rate: f32,
    loan_origination_fee_rate: f32,
    maint_asset_weight: f32,
    init_asset_weight: f32,
    maint_liab_weight: f32,
    init_liab_weight: f32,
    liquidation_fee: f32,
) -> Result<()> {
    // Require token 0 to be in the insurance token
    if token_index == QUOTE_TOKEN_INDEX {
        require_keys_eq!(
            ctx.accounts.group.load()?.insurance_mint,
            ctx.accounts.mint.key()
        );
    }

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        group: ctx.accounts.group.key(),
        name: fill16_from_str(name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config,
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        cached_indexed_total_deposits: I80F48::ZERO,
        cached_indexed_total_borrows: I80F48::ZERO,
        indexed_deposits: I80F48::ZERO,
        indexed_borrows: I80F48::ZERO,
        index_last_updated: Clock::get()?.unix_timestamp,
        bank_rate_last_updated: Clock::get()?.unix_timestamp,
        // TODO: add a require! verifying relation between the parameters
        avg_utilization: I80F48::ZERO,
        adjustment_factor: I80F48::from_num(interest_rate_params.adjustment_factor),
        util0: I80F48::from_num(interest_rate_params.util0),
        rate0: I80F48::from_num(interest_rate_params.rate0),
        util1: I80F48::from_num(interest_rate_params.util1),
        rate1: I80F48::from_num(interest_rate_params.rate1),
        max_rate: I80F48::from_num(interest_rate_params.max_rate),
        collected_fees_native: I80F48::ZERO,
        loan_origination_fee_rate: I80F48::from_num(loan_origination_fee_rate),
        loan_fee_rate: I80F48::from_num(loan_fee_rate),
        maint_asset_weight: I80F48::from_num(maint_asset_weight),
        init_asset_weight: I80F48::from_num(init_asset_weight),
        maint_liab_weight: I80F48::from_num(maint_liab_weight),
        init_liab_weight: I80F48::from_num(init_liab_weight),
        liquidation_fee: I80F48::from_num(liquidation_fee),
        dust: I80F48::ZERO,
        flash_loan_vault_initial: u64::MAX,
        flash_loan_approved_amount: 0,
        token_index,
        bump: *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        reserved: [0; 256],
    };

    let mut mint_info = ctx.accounts.mint_info.load_init()?;
    *mint_info = MintInfo {
        group: ctx.accounts.group.key(),
        token_index,
        padding1: Default::default(),
        mint: ctx.accounts.mint.key(),
        banks: Default::default(),
        vaults: Default::default(),
        oracle: ctx.accounts.oracle.key(),
        registration_time: Clock::get()?.unix_timestamp,
        group_insurance_fund: 1,
        reserved: [0; 255],
    };

    mint_info.banks[0] = ctx.accounts.bank.key();
    mint_info.vaults[0] = ctx.accounts.vault.key();

    Ok(())
}
