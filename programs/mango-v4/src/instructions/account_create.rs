use anchor_lang::prelude::*;

use crate::state::*;
use crate::util::fill_from_str;

pub fn account_create(
    account_ai: &AccountLoader<MangoAccountFixed>,
    account_bump: u8,
    group: Pubkey,
    owner: Pubkey,
    account_num: u32,
    token_count: u8,
    serum3_count: u8,
    perp_count: u8,
    perp_oo_count: u8,
    token_conditional_swap_count: u8,
    name: String,
) -> Result<()> {
    let mut account = account_ai.load_full_init()?;

    let header = MangoAccountDynamicHeader {
        token_count,
        serum3_count,
        perp_count,
        perp_oo_count,
        token_conditional_swap_count,
    };
    header.check_resize_from(&MangoAccountDynamicHeader::zero())?;

    msg!(
        "Initialized account with header version {}",
        account.header_version()
    );

    account.fixed.name = fill_from_str(&name)?;
    account.fixed.group = group;
    account.fixed.owner = owner;
    account.fixed.account_num = account_num;
    account.fixed.bump = account_bump;
    account.fixed.delegate = Pubkey::default();
    account.fixed.set_being_liquidated(false);

    account.resize_dynamic_content(
        token_count,
        serum3_count,
        perp_count,
        perp_oo_count,
        token_conditional_swap_count,
    )?;

    Ok(())
}
