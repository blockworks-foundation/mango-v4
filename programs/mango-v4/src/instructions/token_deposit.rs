use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::health::*;
use crate::state::*;
use crate::util::checked_math as cm;

use crate::logs::{DepositLog, TokenBalanceLog};

// Same as TokenDeposit, but without the owner signing
#[derive(Accounts)]
pub struct TokenDepositIntoExisting<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TokenDeposit) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TokenDeposit<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::TokenDeposit) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

struct DepositCommon<'a, 'info> {
    pub group: &'a AccountLoader<'info, Group>,
    pub account: &'a AccountLoader<'info, MangoAccountFixed>,
    pub bank: &'a AccountLoader<'info, Bank>,
    pub vault: &'a Account<'info, TokenAccount>,
    pub oracle: &'a UncheckedAccount<'info>,
    pub token_account: &'a Box<Account<'info, TokenAccount>>,
    pub token_authority: &'a Signer<'info>,
    pub token_program: &'a Program<'info, Token>,
}

impl<'a, 'info> DepositCommon<'a, 'info> {
    fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.token_account.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.token_authority.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }

    fn deposit_into_existing(
        &self,
        remaining_accounts: &[AccountInfo],
        amount: u64,
        reduce_only: bool,
        allow_token_account_closure: bool,
    ) -> Result<()> {
        require_msg!(amount > 0, "deposit amount must be positive");

        let mut bank = self.bank.load_mut()?;
        let token_index = bank.token_index;

        let amount_i80f48 = {
            // Get the account's position for that token index
            let account = self.account.load_full()?;
            let position = account.token_position(token_index)?;

            let amount_i80f48 = if reduce_only || bank.is_reduce_only() {
                position
                    .native(&bank)
                    .min(I80F48::ZERO)
                    .abs()
                    .ceil()
                    .min(I80F48::from(amount))
            } else {
                I80F48::from(amount)
            };
            if bank.is_reduce_only() {
                require!(
                    reduce_only || amount_i80f48 == I80F48::from(amount),
                    MangoError::TokenInReduceOnlyMode
                );
            }
            amount_i80f48
        };

        // Get the account's position for that token index
        let mut account = self.account.load_full_mut()?;

        let (position, raw_token_index) = account.token_position_mut(token_index)?;

        let position_is_active = {
            bank.deposit(
                position,
                amount_i80f48,
                Clock::get()?.unix_timestamp.try_into().unwrap(),
            )?
        };

        // Transfer the actual tokens
        token::transfer(self.transfer_ctx(), amount_i80f48.to_num::<u64>())?;

        let indexed_position = position.indexed_position;
        let oracle_price = bank.oracle_price(
            &AccountInfoRef::borrow(self.oracle.as_ref())?,
            None, // staleness checked in health
        )?;

        // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
        let amount_usd = cm!(amount_i80f48 * oracle_price).to_num::<i64>();
        cm!(account.fixed.net_deposits += amount_usd);

        emit!(TokenBalanceLog {
            mango_group: self.group.key(),
            mango_account: self.account.key(),
            token_index,
            indexed_position: indexed_position.to_bits(),
            deposit_index: bank.deposit_index.to_bits(),
            borrow_index: bank.borrow_index.to_bits(),
        });
        drop(bank);

        //
        // Health computation
        //
        let retriever = new_fixed_order_account_retriever(remaining_accounts, &account.borrow())?;
        let cache = new_health_cache(&account.borrow(), &retriever)?;

        // Since depositing can only increase health, we can skip the usual pre-health computation.
        // Also, TokenDeposit is one of the rare instructions that is allowed even during being_liquidated.
        // Being in a health region always means being_liquidated is false, so it's safe to gate the check.
        if !account.fixed.is_in_health_region() {
            let health = cache.health(HealthType::LiquidationEnd);
            msg!("health: {}", health);

            let was_being_liquidated = account.being_liquidated();
            let recovered = account.fixed.maybe_recover_from_being_liquidated(health);
            require!(
                !was_being_liquidated || recovered,
                MangoError::DepositsIntoLiquidatingMustRecover
            );
        }

        // Group level deposit limit on account
        let group = self.group.load()?;
        if group.deposit_limit_quote > 0 {
            let assets = cache
                .health_assets_and_liabs(HealthType::Init)
                .0
                .round_to_zero()
                .checked_to_num::<u64>()
                .unwrap();
            require_msg_typed!(
                assets <= group.deposit_limit_quote,
                MangoError::DepositLimit,
                "assets ({}) can't cross deposit limit on the group ({})",
                assets,
                group.deposit_limit_quote
            );
        }

        //
        // Deactivate the position only after the health check because the user passed in
        // remaining_accounts for all banks/oracles, including the account that will now be
        // deactivated.
        // Deposits can deactivate a position if they cancel out a previous borrow.
        //
        if allow_token_account_closure && !position_is_active {
            account.deactivate_token_position_and_log(raw_token_index, self.account.key());
        }

        emit!(DepositLog {
            mango_group: self.group.key(),
            mango_account: self.account.key(),
            signer: self.token_authority.key(),
            token_index,
            quantity: amount_i80f48.to_num::<u64>(),
            price: oracle_price.to_bits(),
        });

        Ok(())
    }
}

pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64, reduce_only: bool) -> Result<()> {
    {
        let token_index = ctx.accounts.bank.load()?.token_index;
        let mut account = ctx.accounts.account.load_full_mut()?;
        account.ensure_token_position(token_index)?;
    }

    DepositCommon {
        group: &ctx.accounts.group,
        account: &ctx.accounts.account,
        bank: &ctx.accounts.bank,
        vault: &ctx.accounts.vault,
        oracle: &ctx.accounts.oracle,
        token_account: &ctx.accounts.token_account,
        token_authority: &ctx.accounts.token_authority,
        token_program: &ctx.accounts.token_program,
    }
    .deposit_into_existing(ctx.remaining_accounts, amount, reduce_only, true)
}

pub fn token_deposit_into_existing(
    ctx: Context<TokenDepositIntoExisting>,
    amount: u64,
    reduce_only: bool,
) -> Result<()> {
    DepositCommon {
        group: &ctx.accounts.group,
        account: &ctx.accounts.account,
        bank: &ctx.accounts.bank,
        vault: &ctx.accounts.vault,
        oracle: &ctx.accounts.oracle,
        token_account: &ctx.accounts.token_account,
        token_authority: &ctx.accounts.token_authority,
        token_program: &ctx.accounts.token_program,
    }
    .deposit_into_existing(ctx.remaining_accounts, amount, reduce_only, false)
}
