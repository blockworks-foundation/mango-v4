use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use solana_program::pubkey::PUBKEY_BYTES;

use crate::address_lookup_table;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = address_lookup_table,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        constraint = bank.load()?.mint == token_account.mint,
    )]
    pub bank: AccountLoader<'info, TokenBank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_authority: Signer<'info>,

    #[account(mut)]
    pub address_lookup_table: UncheckedAccount<'info>, // TODO: wrapper?

    pub token_program: Program<'info, Token>,
    pub address_lookup_table_program: UncheckedAccount<'info>, // TODO: force address?
}

impl<'info> Deposit<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.token_account.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.token_authority.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

fn address_lookup_table_contains(table: &AccountInfo, pubkey: &Pubkey) -> Result<bool> {
    let table_data = table.try_borrow_data()?;
    let pk_ref = pubkey.as_ref();
    Ok(table_data[address_lookup_table::LOOKUP_TABLE_META_SIZE..]
        .chunks(PUBKEY_BYTES)
        .find(|&d| d == pk_ref)
        .is_some())
}

// TODO: It may make sense to have the token_index passed in from the outside.
//       That would save a lot of computation that needs to go into finding the
//       right index for the mint.
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let (is_new_position, oracle) = {
        // Find the mint's token index
        let group = ctx.accounts.group.load()?;
        let mint = ctx.accounts.token_account.mint;
        let token_index = group.tokens.index_for_mint(&mint)?;

        // Get the account's position for that token index
        let mut account = ctx.accounts.account.load_mut()?;
        let position = account.indexed_positions.get_mut_or_create(token_index)?;
        let is_new_position = !position.is_active();

        // Update the bank and position
        let mut bank = ctx.accounts.bank.load_mut()?;
        bank.deposit(position, amount);

        // Transfer the actual tokens
        token::transfer(ctx.accounts.transfer_ctx(), amount)?;

        (is_new_position, bank.oracle)
    };

    // Do we need to add (oracle, bank) to the user's address lookup table?
    //
    // Since they are always added as a pair, checking for one is sufficient.
    let add_to_lookup_table = is_new_position
        && !address_lookup_table_contains(&ctx.accounts.address_lookup_table, &oracle)?;
    if add_to_lookup_table {
        // NOTE: Unfortunately extend() _requires_ a payer, even though we've already
        // fully funded the address lookup table. No further transfer will be necessary.
        // We'll pass the account as payer.
        let mut instruction = address_lookup_table::extend_lookup_table(
            ctx.accounts.address_lookup_table.key(),
            ctx.accounts.account.key(),
            ctx.accounts.account.key(),
            vec![ctx.accounts.bank.key(), oracle],
        );
        // Sneakily remove the system_program account: that way any attempted transfer would error.
        instruction.accounts.pop();
        let account_infos = [
            ctx.accounts.address_lookup_table.to_account_info(),
            ctx.accounts.account.to_account_info(),
            ctx.accounts.account.to_account_info(),
        ];
        // Signing for the account is complicated because it must work as a payer which means
        // a mutable borrow. Thus we must make copies of the values in the seed.
        struct AccountSeedValues {
            group: Pubkey,
            owner: Pubkey,
            account_num: u8,
            bump: u8,
        }
        let account_seed_values = {
            let account = ctx.accounts.account.load()?;
            AccountSeedValues {
                group: account.group,
                owner: account.owner,
                account_num: account.account_num,
                bump: account.bump,
            }
        };
        let account_seeds = account_seeds!(account_seed_values);
        solana_program::program::invoke_signed(&instruction, &account_infos, &[account_seeds])?;
    }

    Ok(())
}
