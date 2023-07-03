use anchor_lang::prelude::*;

use crate::{accounts_ix::*, state::TokenIndex};

// use case - transfer group ownership to governance, where
// admin and fast_listing_admin are PDAs
#[allow(clippy::too_many_arguments)]
pub fn group_edit(
    ctx: Context<GroupEdit>,
    admin_opt: Option<Pubkey>,
    fast_listing_admin_opt: Option<Pubkey>,
    security_admin_opt: Option<Pubkey>,
    testing_opt: Option<u8>,
    version_opt: Option<u8>,
    deposit_limit_quote_opt: Option<u64>,
    buyback_fees_opt: Option<bool>,
    buyback_fees_bonus_factor_opt: Option<f32>,
    buyback_fees_swap_mango_account_opt: Option<Pubkey>,
    mngo_token_index_opt: Option<TokenIndex>,
    buyback_fees_expiry_interval_opt: Option<u64>,
    token_conditional_swap_taker_fee_fraction_opt: Option<f32>,
    token_conditional_swap_maker_fee_fraction_opt: Option<f32>,
) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;

    if let Some(admin) = admin_opt {
        require_keys_neq!(admin, Pubkey::default());
        msg!("Admin old {:?}, new {:?}", group.admin, admin);
        group.admin = admin;
    }

    if let Some(fast_listing_admin) = fast_listing_admin_opt {
        msg!(
            "Fast listing admin old {:?}, new {:?}",
            group.fast_listing_admin,
            fast_listing_admin
        );
        group.fast_listing_admin = fast_listing_admin;
    }

    if let Some(security_admin) = security_admin_opt {
        msg!(
            "Security admin old {:?}, new {:?}",
            group.security_admin,
            security_admin
        );
        group.security_admin = security_admin;
    }

    if let Some(testing) = testing_opt {
        msg!("Testing old {:?}, new {:?}", group.testing, testing);
        group.testing = testing;
    }

    if let Some(version) = version_opt {
        msg!("Version old {:?}, new {:?}", group.version, version);
        group.version = version;
    }

    if let Some(deposit_limit_quote) = deposit_limit_quote_opt {
        msg!(
            "Deposit limit quote old {:?}, new {:?}",
            group.deposit_limit_quote,
            deposit_limit_quote
        );
        group.deposit_limit_quote = deposit_limit_quote;
    }

    if let Some(buyback_fees) = buyback_fees_opt {
        msg!(
            "Buyback fees old {:?}, new {:?}",
            group.buyback_fees,
            buyback_fees
        );
        group.buyback_fees = u8::from(buyback_fees);
    }
    if let Some(buyback_fees_mngo_bonus_factor) = buyback_fees_bonus_factor_opt {
        msg!(
            "Buyback fees mngo bonus factor old {:?}, new {:?}",
            group.buyback_fees_mngo_bonus_factor,
            buyback_fees_mngo_bonus_factor
        );
        group.buyback_fees_mngo_bonus_factor = buyback_fees_mngo_bonus_factor;
    }
    if let Some(buyback_fees_swap_mango_account) = buyback_fees_swap_mango_account_opt {
        msg!(
            "Buyback fees swap mango account old {:?}, new {:?}",
            group.buyback_fees_swap_mango_account,
            buyback_fees_swap_mango_account
        );
        group.buyback_fees_swap_mango_account = buyback_fees_swap_mango_account;
    }
    if let Some(mngo_token_index) = mngo_token_index_opt {
        msg!(
            "Mngo token index old {:?}, new {:?}",
            group.mngo_token_index,
            mngo_token_index
        );
        group.mngo_token_index = mngo_token_index;
    }

    if let Some(buyback_fees_expiry_interval) = buyback_fees_expiry_interval_opt {
        msg!(
            "Buyback fees expiry interval old {:?}, new {:?}",
            group.buyback_fees_expiry_interval,
            buyback_fees_expiry_interval
        );
        group.buyback_fees_expiry_interval = buyback_fees_expiry_interval;
    }

    if let Some(fee_fraction) = token_conditional_swap_taker_fee_fraction_opt {
        msg!(
            "Token conditional swap taker fee fraction old {:?}, new {:?}",
            group.token_conditional_swap_taker_fee_fraction,
            fee_fraction
        );
        require_gte!(fee_fraction, 0.0); // values <0 are not currently supported
        group.token_conditional_swap_taker_fee_fraction = fee_fraction;
    }
    if let Some(fees_fraction) = token_conditional_swap_maker_fee_fraction_opt {
        msg!(
            "Token conditional swap maker fee fraction old {:?}, new {:?}",
            group.token_conditional_swap_maker_fee_fraction,
            fees_fraction
        );
        require_gte!(fees_fraction, 0.0); // values <0 are not currently supported
        group.token_conditional_swap_maker_fee_fraction = fees_fraction;
    }

    Ok(())
}
