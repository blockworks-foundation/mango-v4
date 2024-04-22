use crate::util::fill_from_str;
use crate::{accounts_ix::*, error::MangoError};
use anchor_lang::prelude::*;

pub fn openbook_v2_edit_market(
    ctx: Context<OpenbookV2EditMarket>,
    reduce_only_opt: Option<bool>,
    force_close_opt: Option<bool>,
    name_opt: Option<String>,
    oracle_price_band_opt: Option<f32>,
) -> Result<()> {
    let mut openbook_market = ctx.accounts.market.load_mut()?;

    let group = ctx.accounts.group.load()?;
    let mut require_group_admin = false;

    if let Some(reduce_only) = reduce_only_opt {
        msg!(
            "Reduce only: old - {:?}, new - {:?}",
            openbook_market.reduce_only,
            u8::from(reduce_only)
        );
        openbook_market.reduce_only = u8::from(reduce_only);

        // security admin can only enable reduce_only
        if !reduce_only {
            require_group_admin = true;
        }
    };

    if let Some(force_close) = force_close_opt {
        if force_close {
            require!(openbook_market.is_reduce_only(), MangoError::SomeError);
        }
        msg!(
            "Force close: old - {:?}, new - {:?}",
            openbook_market.force_close,
            u8::from(force_close)
        );
        openbook_market.force_close = u8::from(force_close);
        require_group_admin = true;
    };

    if let Some(name) = name_opt.as_ref() {
        msg!("Name: old - {:?}, new - {:?}", openbook_market.name, name);
        openbook_market.name = fill_from_str(&name)?;
        require_group_admin = true;
    };

    if let Some(oracle_price_band) = oracle_price_band_opt {
        msg!(
            "Oracle price band: old - {:?}, new - {:?}",
            openbook_market.oracle_price_band,
            oracle_price_band
        );
        openbook_market.oracle_price_band = oracle_price_band;
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
