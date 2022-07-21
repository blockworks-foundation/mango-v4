use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;

use crate::error::*;
use crate::state::*;
use crate::util::checked_math as cm;

use crate::logs::{DepositLog, TokenBalanceLog};

#[derive(Accounts)]
pub struct TokenDeposit<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        has_one = vault,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    pub token_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub account2: UncheckedAccount<'info>,
}

impl<'info> TokenDeposit<'info> {
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
pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64) -> Result<()> {
    require_msg!(amount > 0, "deposit amount must be positive");

    let token_index = ctx.accounts.bank.load()?.token_index;

    // Get the account's position for that token index
    let mut account = ctx.accounts.account.load_mut()?;
    require!(!account.is_bankrupt(), MangoError::IsBankrupt);

    let (position, raw_token_index, active_token_index) =
        account.tokens.get_mut_or_create(token_index)?;

    let amount_i80f48 = I80F48::from(amount);
    let position_is_active = {
        let mut bank = ctx.accounts.bank.load_mut()?;
        bank.deposit(position, amount_i80f48)?
    };

    // Transfer the actual tokens
    token::transfer(ctx.accounts.transfer_ctx(), amount)?;

    let indexed_position = position.indexed_position;

    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account)?;
    let (bank, oracle_price) =
        retriever.bank_and_oracle(&ctx.accounts.group.key(), active_token_index, token_index)?;

    // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
    account.net_deposits += cm!(amount_i80f48 * oracle_price * QUOTE_NATIVE_TO_UI).to_num::<f32>();

    emit!(TokenBalanceLog {
        mango_account: ctx.accounts.account.key(),
        token_index,
        indexed_position: indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
        price: oracle_price.to_bits(),
    });

    //
    // Health computation
    // TODO: This will be used to disable is_bankrupt or being_liquidated
    //       when health recovers sufficiently
    //
    let health = compute_health(&account, HealthType::Init, &retriever)
        .context("post-deposit init health")?;
    msg!("health: {}", health);

    //
    // Deactivate the position only after the health check because the user passed in
    // remaining_accounts for all banks/oracles, including the account that will now be
    // deactivated.
    // Deposits can deactivate a position if they cancel out a previous borrow.
    //
    if !position_is_active {
        account.tokens.deactivate(raw_token_index);
    }

    emit!(DepositLog {
        mango_account: ctx.accounts.account.key(),
        signer: ctx.accounts.token_authority.key(),
        token_index: token_index,
        quantity: amount,
        price: oracle_price.to_bits(),
    });

    //
    // mango account 2 i.e. mango account with expandable positions
    //
    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new(&ctx.accounts.account2)?;
    let mut meta = mal.load_mut()?;
    let mut token_position = meta.token_raw_mut(0);
    token_position.in_use_count = 1;

    Ok(())
}
