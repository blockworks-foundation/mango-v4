use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fixed::types::I80F48;

use crate::error::*;
use crate::instructions::INDEX_START;
use crate::state::*;
use crate::util::fill16_from_str;

const FIRST_BANK_NUM: u32 = 0;

#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct TokenRegisterTrustless<'info> {
    #[account(
        has_one = fast_listing_admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub fast_listing_admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [group.key().as_ref(), b"Bank".as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"Vault".as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
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

#[allow(clippy::too_many_arguments)]
pub fn token_register_trustless(
    ctx: Context<TokenRegisterTrustless>,
    token_index: TokenIndex,
    name: String,
) -> Result<()> {
    require_neq!(token_index, 0);

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        group: ctx.accounts.group.key(),
        name: fill16_from_str(name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config: OracleConfig {
            conf_filter: I80F48::from_num(0.10),
        },
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        cached_indexed_total_deposits: I80F48::ZERO,
        cached_indexed_total_borrows: I80F48::ZERO,
        indexed_deposits: I80F48::ZERO,
        indexed_borrows: I80F48::ZERO,
        index_last_updated: Clock::get()?.unix_timestamp,
        bank_rate_last_updated: Clock::get()?.unix_timestamp,
        avg_utilization: I80F48::ZERO,
        // 10% daily adjustment at 0% or 100% utilization
        adjustment_factor: I80F48::from_num(0.004),
        util0: I80F48::from_num(0.7),
        rate0: I80F48::from_num(0.1),
        util1: I80F48::from_num(0.85),
        rate1: I80F48::from_num(0.2),
        max_rate: I80F48::from_num(2.0),
        collected_fees_native: I80F48::ZERO,
        loan_origination_fee_rate: I80F48::from_num(0.0005),
        loan_fee_rate: I80F48::from_num(0.005),
        maint_asset_weight: I80F48::from_num(0),
        init_asset_weight: I80F48::from_num(0),
        maint_liab_weight: I80F48::from_num(1.4), // 2.5x
        init_liab_weight: I80F48::from_num(1.8),  // 1.25x
        liquidation_fee: I80F48::from_num(0.2),
        dust: I80F48::ZERO,
        flash_loan_token_account_initial: u64::MAX,
        flash_loan_approved_amount: 0,
        token_index,
        bump: *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        reserved: [0; 256],
    };
    require_gt!(bank.max_rate, MINIMUM_MAX_RATE);

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
        group_insurance_fund: 0,
        reserved: [0; 255],
    };

    mint_info.banks[0] = ctx.accounts.bank.key();
    mint_info.vaults[0] = ctx.accounts.vault.key();

    Ok(())
}
