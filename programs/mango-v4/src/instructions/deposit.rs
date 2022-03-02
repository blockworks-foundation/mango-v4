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

    update_bank_and_oracle_in_alt(
        ctx.accounts.address_lookup_table.to_account_info(),
        &ctx.accounts.account,
        &ctx.accounts.bank.key(),
        &oracle,
        !position_was_active && position_is_active,
        position_was_active && !position_is_active,
        position_index,
        old_position_len,
    )?;

    Ok(())
}

pub fn update_bank_and_oracle_in_alt<'info>(
    lookup_table_ai: AccountInfo<'info>,
    account_ai: &AccountLoader<'info, MangoAccount>,
    bank: &Pubkey,
    oracle: &Pubkey,
    is_new: bool,
    is_removed: bool,
    position_index: usize,
    old_position_len: usize,
) -> Result<()> {
    // If the position is newly active, add (bank, oracle) to the address lookup table selection,
    // and maybe to the address lookup table, if they're not on there already.
    if is_new {
        // maybe add to lookup table?
        let existing_opt = address_lookup_table::addresses(&lookup_table_ai.try_borrow_data()?)
            .iter()
            .position(|a| a == bank);
        let bank_pos = if let Some(existing) = existing_opt {
            existing
        } else {
            let old_lookup_table_size =
                address_lookup_table::addresses(&lookup_table_ai.try_borrow_data()?).len();

            add_to_alt(lookup_table_ai, account_ai, vec![*bank, *oracle])?;

            old_lookup_table_size
        };

        // Add to lookup table selection
        let mut account = account_ai.load_mut()?;
        // insert the oracle (right to left insertion, to not confuse incides)
        add_to_alt_selection(
            &mut account,
            old_position_len + position_index,
            bank_pos as u8 + 1,
        );
        // insert the bank
        add_to_alt_selection(&mut account, position_index, bank_pos as u8);
    } else if is_removed {
        // Remove from lookup table selection
        let mut account = account_ai.load_mut()?;
        // remove the oracle (right to left removal, to not confuse incides)
        remove_from_alt_selection(&mut account, old_position_len + position_index);
        // remove the bank
        remove_from_alt_selection(&mut account, position_index);
    }

    Ok(())
}

pub fn add_to_alt<'info>(
    lookup_table_ai: AccountInfo<'info>,
    account_ai: &AccountLoader<'info, MangoAccount>,
    new_addresses: Vec<Pubkey>,
) -> Result<()> {
    // NOTE: Unfortunately extend() _requires_ a payer, even though we've already
    // fully funded the address lookup table. No further transfer will be necessary.
    // We'll pass the account as payer.
    let mut instruction = address_lookup_table::extend_lookup_table(
        lookup_table_ai.key(),
        account_ai.key(),
        account_ai.key(),
        new_addresses,
    );
    // Sneakily remove the system_program account: that way any attempted transfer would error.
    instruction.accounts.pop();
    let account_infos = [
        lookup_table_ai,
        account_ai.to_account_info(),
        account_ai.to_account_info(),
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
        let account = account_ai.load()?;
        AccountSeedValues {
            group: account.group,
            owner: account.owner,
            account_num: account.account_num,
            bump: account.bump,
        }
    };
    let account_seeds = account_seeds!(account_seed_values);
    solana_program::program::invoke_signed(&instruction, &account_infos, &[account_seeds])?;
    Ok(())
}

pub fn add_to_alt_selection(account: &mut MangoAccount, at: usize, new_index: u8) {
    let selection = &mut account.address_lookup_table_selection;
    selection[at..].rotate_right(1);
    selection[at] = new_index;
    account.address_lookup_table_selection_size += 1;
}

pub fn remove_from_alt_selection(account: &mut MangoAccount, at: usize) {
    let selection = &mut account.address_lookup_table_selection;
    selection[at..].rotate_left(1);
    account.address_lookup_table_selection_size -= 1;
}
