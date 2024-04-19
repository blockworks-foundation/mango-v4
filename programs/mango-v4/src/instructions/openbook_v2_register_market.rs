use anchor_lang::prelude::*;

use crate::error::*;
use crate::state::*;
use crate::util::fill_from_str;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, OpenbookV2RegisterMarketLog};

pub fn openbook_v2_register_market(
    ctx: Context<OpenbookV2RegisterMarket>,
    market_index: OpenbookV2MarketIndex,
    name: String,
    oracle_price_band: f32,
) -> Result<()> {
    let is_fast_listing;
    let group = ctx.accounts.group.load()?;
    // checking the admin account (#1)
    if ctx.accounts.admin.key() == group.admin {
        is_fast_listing = false;
    } else if ctx.accounts.admin.key() == group.fast_listing_admin {
        is_fast_listing = true;
    } else {
        return Err(error_msg!(
            "admin must be the group admin or group fast listing admin"
        ));
    }

    let base_bank = ctx.accounts.base_bank.load()?;
    let quote_bank = ctx.accounts.quote_bank.load()?;
    let market_external = ctx.accounts.openbook_v2_market_external.load()?;
    require_keys_eq!(
        market_external.quote_mint,
        quote_bank.mint,
        MangoError::SomeError
    );
    require_keys_eq!(
        market_external.base_mint,
        base_bank.mint,
        MangoError::SomeError
    );

    if is_fast_listing {
        // C tier tokens (no borrows, no asset weight) allow wider bands if the quote token has
        // no deposit limits
        let base_c_tier =
            base_bank.are_borrows_reduce_only() && base_bank.maint_asset_weight.is_zero();
        let quote_has_no_deposit_limit = quote_bank.deposit_weight_scale_start_quote == f64::MAX
            && quote_bank.deposit_limit == 0;
        if base_c_tier && quote_has_no_deposit_limit {
            require_eq!(oracle_price_band, 19.0);
        } else {
            require_eq!(oracle_price_band, 1.0);
        }
    }

    let mut openbook_market = ctx.accounts.openbook_v2_market.load_init()?;
    *openbook_market = OpenbookV2Market {
        group: ctx.accounts.group.key(),
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        reduce_only: 0,
        force_close: 0,
        name: fill_from_str(&name)?,
        openbook_v2_program: ctx.accounts.openbook_v2_program.key(),
        openbook_v2_market_external: ctx.accounts.openbook_v2_market_external.key(),
        market_index,
        bump: *ctx
            .bumps
            .get("openbook_v2_market")
            .ok_or(MangoError::SomeError)?,
        oracle_price_band,
        registration_time: Clock::get()?.unix_timestamp.try_into().unwrap(),
        reserved: [0; 1027],
    };

    let mut openbook_index_reservation = ctx.accounts.index_reservation.load_init()?;
    *openbook_index_reservation = OpenbookV2MarketIndexReservation {
        group: ctx.accounts.group.key(),
        market_index,
        reserved: [0; 38],
    };

    emit_stack(OpenbookV2RegisterMarketLog {
        mango_group: ctx.accounts.group.key(),
        openbook_market: ctx.accounts.openbook_v2_market.key(),
        market_index,
        base_token_index: base_bank.token_index,
        quote_token_index: quote_bank.token_index,
        openbook_program: ctx.accounts.openbook_v2_program.key(),
        openbook_market_external: ctx.accounts.openbook_v2_market_external.key(),
    });

    Ok(())
}
