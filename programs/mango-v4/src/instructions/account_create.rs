use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill32_from_str;
use fixed_macro::types::I80F48;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct AccountCreate<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"MangoAccount".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MangoAccount>(),
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,

    #[account(
        mut,
        seeds = [group.key().as_ref(), b"MangoAccount2".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump
    )]
    // borsh smashes the stack, and zero copy doesnt work out of the box
    // deserialize manually
    pub account2: UncheckedAccount<'info>,
}

pub fn account_create(ctx: Context<AccountCreate>, account_num: u8, name: String) -> Result<()> {
    let mut account = ctx.accounts.account.load_init()?;

    account.name = fill32_from_str(name)?;
    account.group = ctx.accounts.group.key();
    account.owner = ctx.accounts.owner.key();
    account.account_num = account_num;
    account.bump = *ctx.bumps.get("account").ok_or(MangoError::SomeError)?;
    account.delegate = Pubkey::default();
    account.tokens = MangoAccountTokenPositions::default();
    account.serum3 = MangoAccountSerum3Orders::default();
    account.perps = MangoAccountPerpPositions::default();
    account.set_being_liquidated(false);
    account.set_bankrupt(false);

    //
    // mango account 2 i.e. mango account with expandable positions
    //
    let token_count: usize = 3;
    let serum3_count: usize = 4;
    let perp_count: usize = 2;
    // create account
    {
        let space = MangoAccount2::space(token_count, serum3_count, perp_count);
        let lamports = Rent::get()?.minimum_balance(space);
        let cpi_accounts = anchor_lang::system_program::CreateAccount {
            from: ctx.accounts.payer.to_account_info(),
            to: ctx.accounts.account2.to_account_info(),
        };
        let cpi_context = anchor_lang::context::CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            cpi_accounts,
        );
        let (__pda_address, __bump) = Pubkey::find_program_address(
            &[
                ctx.accounts.group.key().as_ref(),
                b"MangoAccount2".as_ref(),
                ctx.accounts.owner.key().as_ref(),
                &account_num.to_le_bytes(),
            ],
            ctx.program_id,
        );
        anchor_lang::system_program::create_account(
            cpi_context.with_signer(&[&[
                ctx.accounts.group.key().as_ref(),
                b"MangoAccount2".as_ref(),
                ctx.accounts.owner.key().as_ref(),
                &account_num.to_le_bytes(),
                &[__bump][..],
            ][..]]),
            lamports,
            space as u64,
            ctx.program_id,
        )?;
    }
    // init disc, dynamic fields lengths
    let mal: MangoAccountLoader<MangoAccount2Fixed, MangoAccount2DynamicHeader, MangoAccount2> =
        MangoAccountLoader::new(ctx.accounts.account2.to_account_info());
    let mut meta = mal.load_init(
        // &ctx.accounts.account2,
        token_count,
        serum3_count,
        perp_count,
    )?;
    meta.dynamic.write_tokens_length();
    meta.dynamic.write_serum3_length();
    meta.dynamic.write_perp_length();
    // init fixed fields
    // later we would expand, and verify if the existing ones are set and new expanded ones are unset
    meta.fixed.owner = ctx.accounts.owner.key();
    // test
    for i in 0..3 {
        let pos = meta.dynamic.token_get_raw_mut(i);
        pos.token_index = i as TokenIndex + 1;
    }
    for i in 0..4 {
        let pos = meta.dynamic.serum3_get_raw_mut(i);
        pos.market_index = i as Serum3MarketIndex + 1;
    }
    for i in 0..2 {
        let pos = meta.dynamic.perp_get_raw_mut(i);
        pos.market_index = i as PerpMarketIndex + 1;
    }

    Ok(())
}
