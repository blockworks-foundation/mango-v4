use std::mem;

use anchor_lang::prelude::*;

use crate::error::*;
use crate::mango_account_loader::*;
use crate::state::*;
use crate::util::fill32_from_str;
use anchor_lang::Discriminator;

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

    #[account(mut)]
    pub em_test_account: UncheckedAccount<'info>,
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
    //

    msg!("header version {}", mem::align_of::<u8>());
    msg!("header {}", mem::align_of::<EMTestAccountHeader>());
    msg!("TokenPosition {}", mem::align_of::<TokenPosition>());

    let space = 8
        + mem::size_of::<EMTestAccount>()
        + (1 + mem::size_of::<EMTestAccountHeader>() + 8
            - (1 + mem::size_of::<EMTestAccountHeader>() % 8))
        + mem::size_of::<TokenPosition>();
    msg!("space {:?}", space);
    let lamports = Rent::get()?.minimum_balance(space);
    let cpi_accounts = anchor_lang::system_program::CreateAccount {
        from: ctx.accounts.payer.to_account_info(),
        to: ctx.accounts.em_test_account.to_account_info(),
    };
    let cpi_context = anchor_lang::context::CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        cpi_accounts,
    );
    let (__pda_address, __bump) = Pubkey::find_program_address(
        &[
            ctx.accounts.group.key().as_ref(),
            b"EMTestAccount".as_ref(),
            ctx.accounts.owner.key().as_ref(),
            &account_num.to_le_bytes(),
        ],
        ctx.program_id,
    );
    anchor_lang::system_program::create_account(
        cpi_context.with_signer(&[&[
            ctx.accounts.group.key().as_ref(),
            b"EMTestAccount".as_ref(),
            ctx.accounts.owner.key().as_ref(),
            &account_num.to_le_bytes(),
            &[__bump][..],
        ][..]]),
        lamports,
        space as u64,
        ctx.program_id,
    )?;

    //
    //
    let mut data = ctx.accounts.em_test_account.try_borrow_mut_data()?;
    msg!("data.len {:?}", data.len());
    let dst: &mut [u8] = &mut data[0..8];
    dst.copy_from_slice(&EMTestAccount::discriminator());

    drop(data);

    //
    //
    ctx.accounts.em_test_account.try_borrow_mut_data()?[8 + mem::size_of::<EMTestAccount>()] = 0;

    //
    //
    let mut data = ctx.accounts.em_test_account.try_borrow_mut_data()?;
    let dst: &mut [u8] = &mut data[8 + mem::size_of::<EMTestAccount>() + 1
        ..8 + mem::size_of::<EMTestAccount>() + 1 + mem::size_of::<EMTestAccountHeader>()];
    dst.copy_from_slice(bytemuck::bytes_of(&EMTestAccountHeader {
        header_size: 2,
        token_count: 1,
    }));

    drop(data);

    Ok(())
}
