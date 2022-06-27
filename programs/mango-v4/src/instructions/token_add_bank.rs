use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

// TODO: ALTs are unavailable
//use crate::address_lookup_table;

use crate::state::*;

#[derive(Accounts)]
#[instruction(token_index: TokenIndex, bank_num: u64)]
pub struct TokenAddBank<'info> {
    #[account(
        has_one = admin,
    )]
    pub group: AccountLoader<'info, Group>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        constraint = existing_bank.load()?.token_index == token_index,
        has_one = group
    )]
    pub existing_bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [group.key().as_ref(), b"Bank".as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"Vault".as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        token::authority = group,
        token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"MintInfo".as_ref(), mint.key().as_ref()],
        bump
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    // Creating an address lookup table needs a recent valid slot as an
    // input argument. That makes creating ALTs from governance instructions
    // impossible. Hence the ALT that this instruction uses must be created
    // externally and the admin is responsible for placing banks/oracles into
    // sensible address lookup tables.
    // constraint: must be created, have the admin authority and have free space
    // TODO: ALTs are unavailable
    //#[account(mut)]
    //pub address_lookup_table: UncheckedAccount<'info>, // TODO: wrapper?
    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    // TODO: ALTs are unavailable
    //pub address_lookup_table_program: UncheckedAccount<'info>, // TODO: force address?
    pub rent: Sysvar<'info, Rent>,
}

// TODO: should this be "configure_mint", we pass an explicit index, and allow
// overwriting config as long as the mint account stays the same?
#[allow(clippy::too_many_arguments)]
pub fn token_add_bank(
    ctx: Context<TokenAddBank>,
    _token_index: TokenIndex,
    bank_num: u64,
) -> Result<()> {
    // TODO: Error if mint is already configured (technically, init of vault will fail)

    let existing_bank = ctx.accounts.existing_bank.load()?;
    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank::from_existing_bank(&existing_bank, ctx.accounts.vault.key(), bank_num);

    // TODO: ALTs are unavailable
    // let alt_previous_size =
    //     address_lookup_table::addresses(&ctx.accounts.address_lookup_table.try_borrow_data()?)
    //         .len();
    // let address_lookup_table = Pubkey::default();
    // let alt_previous_size = 0;

    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    let free_slot = mint_info
        .banks
        .iter()
        .position(|bank| bank == &Pubkey::default())
        .unwrap();
    require_eq!(bank_num as usize, free_slot);
    mint_info.banks[free_slot] = ctx.accounts.bank.key();
    mint_info.vaults[free_slot] = ctx.accounts.vault.key();

    // TODO: ALTs are unavailable
    /*
    address_lookup_table::extend(
        ctx.accounts.address_lookup_table.to_account_info(),
        // TODO: is using the admin as ALT authority a good idea?
        ctx.accounts.admin.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        &[],
        vec![ctx.accounts.bank.key(), ctx.accounts.oracle.key()],
    )?;
    */

    Ok(())
}
