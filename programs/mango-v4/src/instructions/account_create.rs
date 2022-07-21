use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill32_from_str;

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
        init,
        seeds = [group.key().as_ref(), b"MangoAccount2".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = MangoAccount2::space(3, 4, 2),
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
    // init disc
    let mut mal: MangoAccountLoader<MangoAccount2> =
        MangoAccountLoader::new_init(&ctx.accounts.account2)?;
    let mut meta: MangoAccountAccMut = mal.load_mut()?;
    // init fixed fields
    // later we would expand, and verify if the existing ones are set and new expanded ones are unset
    meta.fixed.owner = ctx.accounts.owner.key();
    // init dynamic fields
    let token_count: u8 = 3;
    let serum3_count: u8 = 4;
    let perp_count: u8 = 2;
    meta.expand_dynamic_content(token_count, serum3_count, perp_count)?;
    // test
    for i in 0..3 {
        let pos = meta.token_get_mut_raw(i);
        pos.token_index = i as TokenIndex + 1;
    }
    for i in 0..4 {
        let pos = meta.serum3_get_mut_raw(i);
        pos.market_index = i as Serum3MarketIndex + 1;
    }
    for i in 0..2 {
        let pos = meta.perp_get_mut_raw(i);
        pos.market_index = i as PerpMarketIndex + 1;
    }

    let meta_borrowed = meta.borrow();
    msg!("{}", meta_borrowed.token_get_raw(1).token_index);

    Ok(())
}
