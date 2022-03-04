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
        loan_token_account_owner_bump_seeds: u8,
        amount_to: u64,
    ) -> Result<()> {
        msg!(
            "taking amount({}) loan from mango for mint {:?}",
            amount_from,
            ctx.accounts.mango_token_vault.mint
        );
        token::transfer(ctx.accounts.transfer_from_mango_vault_ctx(), amount_from)?;

        msg!("TODO: do something with the loan");

        msg!(
            "transferring amount({}) loan back to mango for mint {:?}",
            amount_to,
            ctx.accounts.loan_token_account.mint
        );
        let seeds = &[
            b"margintrade".as_ref(),
            &[loan_token_account_owner_bump_seeds],
        ];
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
    pub mango_group: Signer<'info>,

    #[account(mut)]
    pub mango_token_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub loan_token_account: Account<'info, TokenAccount>,

    // todo: can we do better than UncheckedAccount?
    pub loan_token_account_owner: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> MarginTradeCtx<'info> {
    pub fn transfer_from_mango_vault_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = Transfer {
            from: self.mango_token_vault.to_account_info(),
            to: self.loan_token_account.to_account_info(),
            authority: self.mango_group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }

    pub fn transfer_back_to_mango_vault_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = Transfer {
            from: self.loan_token_account.to_account_info(),
            to: self.mango_token_vault.to_account_info(),
            authority: self.loan_token_account_owner.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
