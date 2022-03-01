use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;

use crate::error::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

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

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.vault.to_account_info(),
            to: self.token_account.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}

// TODO: It may make sense to have the token_index passed in from the outside.
//       That would save a lot of computation that needs to go into finding the
//       right index for the mint.
pub fn withdraw(ctx: Context<Withdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
    // Find the mint's token index
    let group = ctx.accounts.group.load()?;
    let mint = ctx.accounts.token_account.mint;
    let token_index = group.tokens.index_for_mint(&mint)?;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;
    let position = account.indexed_positions.get_mut_or_create(token_index)?.0;

    // The bank will also be passed in remainingAccounts. Use an explicit scope
    // to drop the &mut before we borrow it immutably again later.
    {
        let mut bank = ctx.accounts.bank.load_mut()?;
        let native_position = position.native(&bank);

        // Handle amount special case for withdrawing everything
        let amount = if amount == u64::MAX && !allow_borrow {
            if native_position.is_positive() {
                // TODO: This rounding may mean that if we deposit and immediately withdraw
                //       we can't withdraw the full amount!
                native_position.floor().to_num::<u64>()
            } else {
                return Ok(());
            }
        } else {
            amount
        };

        require!(
            allow_borrow || amount < native_position,
            MangoError::SomeError
        );

        // Update the bank and position
        bank.withdraw(position, amount);

        // Transfer the actual tokens
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            amount,
        )?;
    }

    //
    // Health check (WIP)
    //
    let active_len = account.indexed_positions.iter_active().count();
    require!(
        ctx.remaining_accounts.len() == active_len * 2, // banks + oracles
        MangoError::SomeError
    );

    let mut assets = I80F48::ZERO;
    let mut liabilities = I80F48::ZERO; // absolute value
    for (position, (bank_ai, oracle_ai)) in zip!(
        account.indexed_positions.iter_active(),
        ctx.remaining_accounts.iter(),
        ctx.remaining_accounts.iter().skip(active_len)
    ) {
        let bank_loader = AccountLoader::<'_, TokenBank>::try_from(bank_ai)?;
        let bank = bank_loader.load()?;

        // TODO: This assumes banks are passed in order - is that an ok assumption?
        require!(
            bank.token_index == position.token_index,
            MangoError::SomeError
        );

        // converts the token value to the basis token value for health computations
        // TODO: health basis token == USDC?
        let oracle_type = determine_oracle_type(oracle_ai)?;
        require!(bank.oracle == oracle_ai.key(), MangoError::UnexpectedOracle);

        let price = match oracle_type {
            OracleType::Stub => {
                AccountLoader::<'_, StubOracle>::try_from(oracle_ai)?
                    .load()?
                    .price
            }
        };

        let native_basis = position.native(&bank) * price;
        if native_basis.is_positive() {
            assets += bank.init_asset_weight * native_basis;
        } else {
            liabilities -= bank.init_liab_weight * native_basis;
        }
    }
    let health = assets - liabilities;
    msg!("health: {}", health);
    require!(health > 0, MangoError::SomeError);

    Ok(())
}
