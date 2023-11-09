use crate::util::fill_from_str;
use crate::{accounts_ix::*, error::MangoError};
use anchor_lang::prelude::*;

pub fn serum3_edit_market(
    ctx: Context<Serum3EditMarket>,
    reduce_only_opt: Option<bool>,
    force_close_opt: Option<bool>,
    name_opt: Option<String>,
    oracle_price_band_opt: Option<f32>,
) -> Result<()> {
    let mut serum3_market = ctx.accounts.market.load_mut()?;

    let group = ctx.accounts.group.load()?;
    let mut require_group_admin = false;

    if let Some(reduce_only) = reduce_only_opt {
        msg!(
            "Reduce only: old - {:?}, new - {:?}",
            serum3_market.reduce_only,
            u8::from(reduce_only)
        );
        serum3_market.reduce_only = u8::from(reduce_only);

        // security admin can only enable reduce_only
        if !reduce_only {
            require_group_admin = true;
        }
    };

    if let Some(force_close) = force_close_opt {
        if force_close {
            require!(serum3_market.is_reduce_only(), MangoError::SomeError);
        }
        msg!(
            "Force close: old - {:?}, new - {:?}",
            serum3_market.force_close,
            u8::from(force_close)
        );
        serum3_market.force_close = u8::from(force_close);
        require_group_admin = true;
    };

    if let Some(name) = name_opt.as_ref() {
        msg!("Name: old - {:?}, new - {:?}", serum3_market.name, name);
        serum3_market.name = fill_from_str(&name)?;
        require_group_admin = true;
    };

    if let Some(oracle_price_band) = oracle_price_band_opt {
        msg!(
            "Oracle price band: old - {:?}, new - {:?}",
            serum3_market.oracle_price_band,
            oracle_price_band
        );
        serum3_market.oracle_price_band = oracle_price_band;
        require_group_admin = true;
    };

    if require_group_admin {
        require!(
            group.admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    } else {
        require!(
            group.admin == ctx.accounts.admin.key()
                || group.security_admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    }

    Ok(())
}
