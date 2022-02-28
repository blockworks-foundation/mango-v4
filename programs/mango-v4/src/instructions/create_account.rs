use anchor_lang::prelude::*;

use crate::error::*;
use crate::solana_address_lookup_table_instruction;
use crate::state::*;

#[derive(Accounts)]
#[instruction(account_num: u8)]
pub struct CreateAccount<'info> {
    pub group: AccountLoader<'info, MangoGroup>,

    #[account(
        init,
        seeds = [group.key().as_ref(), b"account".as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MangoAccount>(),
    )]
    pub account: AccountLoader<'info, MangoAccount>,

    pub owner: Signer<'info>,

    // We can't use anchor's `init` here because the create_lookup_table instruction
    // expects an unallocated table.
    // Even though this is a PDA, we can't use anchor's `seeds` here because the
    // address must be based on a recent slot hash, and create_lookup_table() will
    // validate in anyway.
    #[account(mut)]
    pub address_lookup_table: UncheckedAccount<'info>, // TODO: wrapper?

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub address_lookup_table_program: UncheckedAccount<'info>, // TODO: force address?
}

pub fn create_account(
    ctx: Context<CreateAccount>,
    account_num: u8,
    address_lookup_table_recent_slot: u64,
) -> Result<()> {
    {
        let mut account = ctx.accounts.account.load_init()?;
        account.group = ctx.accounts.group.key();
        account.owner = ctx.accounts.owner.key();
        account.address_lookup_table = ctx.accounts.address_lookup_table.key();
        account.account_num = account_num;
        account.bump = *ctx.bumps.get("account").ok_or(MangoError::SomeError)?;
    }

    //
    // Setup address lookup tables initial state:
    // - one is active and empty
    // - other one is deacivated
    //
    // TODO: We could save some CU here by not using create_lookup_table():
    //       it - unnecessarily - derives the lookup table address again.
    let (instruction, _expected_adress_map_address) =
        solana_address_lookup_table_instruction::create_lookup_table(
            ctx.accounts.account.key(),
            ctx.accounts.payer.key(),
            address_lookup_table_recent_slot,
        );
    let account_infos = [
        ctx.accounts.address_lookup_table.to_account_info(),
        ctx.accounts.account.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    ];
    // Anchor only sets the discriminator after this function finishes,
    // calling load() right now would cause an error. But we _do_ need an immutable borrow
    // so hack it by calling exit() early (which only sets the discriminator)
    ctx.accounts.account.exit(&crate::id())?;
    let account = ctx.accounts.account.load()?;
    let seeds = account_seeds!(account);
    solana_program::program::invoke_signed(&instruction, &account_infos, &[seeds])?;

    Ok(())
}
