use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;

use crate::address_lookup_table;
use crate::error::*;
use crate::state::*;

const INDEX_START: I80F48 = I80F48!(1_000_000);

#[derive(Accounts)]
pub struct RegisterToken<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub group: AccountLoader<'info, MangoGroup>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"tokenbank".as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<TokenBank>(),
    )]
    pub bank: AccountLoader<'info, TokenBank>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"tokenvault".as_ref(), mint.key().as_ref()],
        bump,
        token::authority = group,
        token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"mintinfo".as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MintInfo>(),
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    pub oracle: UncheckedAccount<'info>,

    // Creating an address lookup table needs a recent valid slot as an
    // input argument. That makes creating ALTs from governance instructions
    // impossible. Hence the ALT that this instruction uses must be created
    // externally and the admin is responsible for placing banks/oracles into
    // sensible address lookup tables.
    // constraint: must be created, have the admin authority and have free space
    #[account(mut)]
    pub address_lookup_table: UncheckedAccount<'info>, // TODO: wrapper?

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub address_lookup_table_program: UncheckedAccount<'info>, // TODO: force address?
    pub rent: Sysvar<'info, Rent>,
}

// TODO: should this be "configure_mint", we pass an explicit index, and allow
// overwriting config as long as the mint account stays the same?
pub fn register_token(
    ctx: Context<RegisterToken>,
    decimals: u8,
    maint_asset_weight: f32,
    init_asset_weight: f32,
    maint_liab_weight: f32,
    init_liab_weight: f32,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;
    // TODO: Error if mint is already configured (techincally, init of vault will fail)
    // TOOD: Error type
    // TODO: Should be a function: Tokens::add() or so
    let token_index = group
        .tokens
        .infos
        .iter()
        .position(|ti| !ti.is_valid())
        .ok_or(MangoError::SomeError)?;
    group.tokens.infos[token_index] = TokenInfo {
        mint: ctx.accounts.mint.key(),
        decimals,
        bank_bump: *ctx.bumps.get("bank").ok_or(MangoError::SomeError)?, // TODO: error
        vault_bump: *ctx.bumps.get("vault").ok_or(MangoError::SomeError)?, // TODO: error
        reserved: [0u8; 30],
    };

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = TokenBank {
        group: ctx.accounts.group.key(),
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        indexed_total_deposits: I80F48::ZERO,
        indexed_total_borrows: I80F48::ZERO,
        maint_asset_weight: I80F48::from_num(maint_asset_weight),
        init_asset_weight: I80F48::from_num(init_asset_weight),
        maint_liab_weight: I80F48::from_num(maint_liab_weight),
        init_liab_weight: I80F48::from_num(init_liab_weight),
        dust: I80F48::ZERO,
        token_index: token_index as TokenIndex,
    };

    let alt_previous_size =
        address_lookup_table::addresses(&ctx.accounts.address_lookup_table.try_borrow_data()?)
            .iter()
            .count();
    let mut mint_info = ctx.accounts.mint_info.load_init()?;
    *mint_info = MintInfo {
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        bank: ctx.accounts.bank.key(),
        address_lookup_table: ctx.accounts.address_lookup_table.key(),
        address_lookup_table_bank_index: alt_previous_size as u8,
        address_lookup_table_oracle_index: alt_previous_size as u8 + 1,
    };

    address_lookup_table::extend(
        ctx.accounts.address_lookup_table.to_account_info(),
        // TODO: is using the admin as ALT authority a good idea?
        ctx.accounts.admin.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        &[],
        vec![ctx.accounts.bank.key(), ctx.accounts.oracle.key()],
    )?;

    Ok(())
}
