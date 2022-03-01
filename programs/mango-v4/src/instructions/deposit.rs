use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

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

// TODO: It may make sense to have the token_index passed in from the outside.
//       That would save a lot of computation that needs to go into finding the
//       right index for the mint.
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let (position_was_active, position_is_active, position_index, old_position_len, oracle) = {
        // Find the mint's token index
        let group = ctx.accounts.group.load()?;
        let mint = ctx.accounts.token_account.mint;
        let token_index = group.tokens.index_for_mint(&mint)?;

        // Get the account's position for that token index
        let mut account = ctx.accounts.account.load_mut()?;
        let old_position_len = account.indexed_positions.iter_active().count();
        let (position, position_index) =
            account.indexed_positions.get_mut_or_create(token_index)?;
        let position_was_active = position.is_active();

        // Update the bank and position
        let mut bank = ctx.accounts.bank.load_mut()?;
        bank.deposit(position, amount);

        let position_is_active = position.is_active();

        // Transfer the actual tokens
        token::transfer(ctx.accounts.transfer_ctx(), amount)?;

        (
            position_was_active,
            position_is_active,
            position_index,
            old_position_len,
            bank.oracle,
        )
    };

    // If the position is newly active, add (bank, oracle) to the address lookup table selection,
    // and maybe to the address lookup table, if they're not on there already.
    if !position_was_active && position_is_active {
        // maybe add to lookup table?
        let existing_opt =
            address_lookup_table::addresses(&ctx.accounts.address_lookup_table.try_borrow_data()?)
                .iter()
                .position(|&a| a == ctx.accounts.bank.key());
        let bank_pos = if let Some(existing) = existing_opt {
            existing
        } else {
            let old_lookup_table_size = address_lookup_table::addresses(
                &ctx.accounts.address_lookup_table.try_borrow_data()?,
            )
            .len();

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

            old_lookup_table_size
        };

        // Add to lookup table selection
        let mut account = ctx.accounts.account.load_mut()?;
        let selection = &mut account.address_lookup_table_selection;
        // insert the bank
        selection[position_index..].rotate_right(1);
        selection[position_index] = bank_pos as u8;
        // insert the oracle
        selection[old_position_len + 1 + position_index..].rotate_right(1);
        selection[old_position_len + 1 + position_index] = bank_pos as u8 + 1;
        account.address_lookup_table_selection_size += 2;
    } else if position_was_active && !position_is_active {
        // Remove from lookup table selection
        let mut account = ctx.accounts.account.load_mut()?;
        let selection = &mut account.address_lookup_table_selection;
        // remove the oracle
        selection[old_position_len + position_index..].rotate_left(1);
        // remove the bank
        selection[position_index..].rotate_left(1);
        account.address_lookup_table_selection_size -= 2;
    }

    Ok(())
}
