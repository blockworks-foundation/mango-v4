use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Token, TokenAccount, Transfer};

declare_id!("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");

#[program]
pub mod margin_trade {
    use super::*;

    pub fn margin_trade(
        ctx: Context<MarginTradeCtx>,
        amount_from: u64,
        deposit_account_owner_bump_seeds: u8,
        amount_to: u64,
    ) -> Result<()> {
        if amount_from > 0 {
            msg!(
                "withdrawing({}) for mint {:?}",
                amount_from,
                ctx.accounts.withdraw_account.mint
            );
            token::transfer(ctx.accounts.transfer_from_mango_vault_ctx(), amount_from)?;
        }

        msg!("TODO: do something with the loan");

        msg!(
            "depositing amount({}) back to mint {:?}",
            amount_to,
            ctx.accounts.deposit_account.mint
        );
        let seeds = &[b"MarginTrade".as_ref(), &[deposit_account_owner_bump_seeds]];
        token::transfer(
            ctx.accounts
                .transfer_back_to_mango_vault_ctx()
                .with_signer(&[seeds]),
            amount_to,
        )?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct MarginTrade;

impl anchor_lang::Id for MarginTrade {
    fn id() -> Pubkey {
        ID
    }
}

#[derive(Accounts)]

pub struct MarginTradeCtx<'info> {
    /// CHECK: Used as authority for withdraw_account
    pub withdraw_account_owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub withdraw_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub deposit_account: Account<'info, TokenAccount>,

    /// CHECK: Used as authority for deposit_account
    pub deposit_account_owner: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> MarginTradeCtx<'info> {
    pub fn transfer_from_mango_vault_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = Transfer {
            from: self.withdraw_account.to_account_info(),
            to: self.deposit_account.to_account_info(),
            authority: self.withdraw_account_owner.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }

    pub fn transfer_back_to_mango_vault_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = Transfer {
            from: self.deposit_account.to_account_info(),
            to: self.withdraw_account.to_account_info(),
            authority: self.deposit_account_owner.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
