use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::instructions::{apply_vault_difference, OODifference};
use crate::logs::{emit_stack, OpenbookV2OpenOrdersBalanceLog};
use crate::serum3_cpi::{OpenOrdersAmounts, OpenOrdersSlim};
use crate::state::*;
use crate::util::clock_now;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use openbook_v2::cpi::Return;
use openbook_v2::state::OpenOrdersAccount;
use openbook_v2::state::{
    Order as OpenbookV2Order, PlaceOrderType as OpenbookV2OrderType, Side as OpenbookV2Side,
    MAX_OPEN_ORDERS,
};

use crate::accounts_ix::*;

pub fn openbook_v2_place_order(
    ctx: Context<OpenbookV2PlaceOrder>,
    order: OpenbookV2Order,
    limit: u8,
) -> Result<()> {
    require_gte!(order.max_base_lots, 0);
    require_gte!(order.max_quote_lots_including_fees, 0);

    let openbook_market = ctx.accounts.openbook_v2_market.load()?;
    require!(
        !openbook_market.is_reduce_only(),
        MangoError::MarketInReduceOnlyMode
    );

    let receiver_token_index = match order.side {
        OpenbookV2Side::Bid => openbook_market.base_token_index,
        OpenbookV2Side::Ask => openbook_market.quote_token_index,
    };
    let payer_token_index = match order.side {
        OpenbookV2Side::Bid => openbook_market.quote_token_index,
        OpenbookV2Side::Ask => openbook_market.base_token_index,
    };

    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load_full()?;
        // account constraint #1
        require!(
            account
                .fixed
                .is_owner_or_delegate(ctx.accounts.authority.key()),
            MangoError::SomeError
        );

        // Validate open_orders #2
        require!(
            account
                .openbook_v2_orders(openbook_market.market_index)?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );
    }
    // Validate bank and vault #3
    let group_key = ctx.accounts.group.key();
    let mut account = ctx.accounts.account.load_full_mut()?;
    let (now_ts, now_slot) = clock_now();
    let retriever = new_fixed_order_account_retriever_with_optional_banks(
        ctx.remaining_accounts,
        &account.borrow(),
        now_slot,
    )?;

    let (_, _, payer_active_index) = account.ensure_token_position(payer_token_index)?;
    let (_, _, receiver_active_index) = account.ensure_token_position(receiver_token_index)?;

    // This verifies that the required banks are available and that their oracles are valid
    let (payer_bank, payer_bank_oracle) =
        retriever.bank_and_oracle(&group_key, payer_active_index, payer_token_index)?;
    let (receiver_bank, receiver_bank_oracle) =
        retriever.bank_and_oracle(&group_key, receiver_active_index, receiver_token_index)?;

    require_keys_eq!(payer_bank.vault, ctx.accounts.payer_vault.key());

    // Validate bank token indexes #4
    require_eq!(
        ctx.accounts.payer_bank.load()?.token_index,
        payer_token_index
    );
    require_eq!(
        ctx.accounts.receiver_bank.load()?.token_index,
        receiver_token_index
    );

    //
    // Pre-health computation
    //
    let mut health_cache = new_health_cache_skipping_missing_banks_and_bad_oracles(
        &account.borrow(),
        &retriever,
        now_ts,
    )
    .context("pre init health")?;

    // The payer and receiver token banks/oracles must be passed and be valid
    health_cache.token_info_index(payer_token_index)?;
    health_cache.token_info_index(receiver_token_index)?;

    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let pre_init_health = account.check_health_pre(&health_cache)?;
        Some(pre_init_health)
    } else {
        None
    };

    drop(retriever);

    // No version check required, bank writable from v1

    //
    // Before-order tracking
    //
    let base_lot_size: u64;
    let quote_lot_size: u64;
    {
        let openbook_market_external = ctx.accounts.openbook_v2_market_external.load()?;
        base_lot_size = openbook_market_external.base_lot_size.try_into().unwrap();
        quote_lot_size = openbook_market_external.quote_lot_size.try_into().unwrap();
    }

    let before_vault = ctx.accounts.payer_vault.amount;
    let before_oo_free_slots;
    let before_had_bids;
    let before_had_asks;
    let before_oo = {
        let open_orders = ctx.accounts.open_orders.load()?;
        before_oo_free_slots = MAX_OPEN_ORDERS - open_orders.all_orders_in_use().count();
        before_had_bids = open_orders.position.bids_base_lots != 0;
        before_had_asks = open_orders.position.asks_base_lots != 0;
        OpenOrdersSlim::from_oo_v2(&open_orders, base_lot_size, quote_lot_size)
    };

    // Provide a readable error message in case the vault doesn't have enough tokens
    let max_base_lots: u64 = order.max_base_lots.try_into().unwrap();
    let max_quote_lots: u64 = order.max_quote_lots_including_fees.try_into().unwrap();

    let needed_amount = match order.side {
        OpenbookV2Side::Ask => {
            (max_base_lots * base_lot_size).saturating_sub(before_oo.native_base_free())
        }
        OpenbookV2Side::Bid => {
            (max_quote_lots * quote_lot_size).saturating_sub(before_oo.native_quote_free())
        }
    };
    if before_vault < needed_amount {
        return err!(MangoError::InsufficentBankVaultFunds).with_context(|| {
            format!(
                "bank vault does not have enough tokens, need {} but have {}",
                needed_amount, before_vault
            )
        });
    }

    // Get price lots before the book gets modified
    let price_lots;
    {
        let bids = ctx.accounts.bids.load_mut()?;
        let asks = ctx.accounts.asks.load_mut()?;
        let order_book = openbook_v2::state::Orderbook { bids, asks };
        price_lots = order.price(now_ts, None, &order_book)?.0;
    }

    //
    // CPI to place order
    //
    let group = ctx.accounts.group.load()?;
    let group_seeds = group_seeds!(group);

    cpi_place_order(ctx.accounts, &[group_seeds], &order, price_lots, limit)?;
    //
    // After-order tracking
    //
    let open_orders = ctx.accounts.open_orders.load()?;
    let after_oo_free_slots = MAX_OPEN_ORDERS - open_orders.all_orders_in_use().count();
    let after_oo = OpenOrdersSlim::from_oo_v2(&open_orders, base_lot_size, quote_lot_size);
    let oo_difference = OODifference::new(&before_oo, &after_oo);

    //
    // Track the highest bid and lowest ask, to be able to evaluate worst-case health even
    // when they cross the oracle
    //
    let openbook = account.openbook_v2_orders_mut(openbook_market.market_index)?;
    if !before_had_bids {
        // The 0 state means uninitialized/no value
        openbook.highest_placed_bid_inv = 0.0;
        openbook.lowest_placed_bid_inv = 0.0
    }
    if !before_had_asks {
        openbook.lowest_placed_ask = 0.0;
        openbook.highest_placed_ask = 0.0;
    }
    // in the normal quote per base units
    let limit_price = price_lots as f64 * quote_lot_size as f64 / base_lot_size as f64;

    let new_order_on_book = after_oo_free_slots != before_oo_free_slots;
    if new_order_on_book {
        match order.side {
            OpenbookV2Side::Ask => {
                openbook.lowest_placed_ask = if openbook.lowest_placed_ask == 0.0 {
                    limit_price
                } else {
                    openbook.lowest_placed_ask.min(limit_price)
                };
                openbook.highest_placed_ask = if openbook.highest_placed_ask == 0.0 {
                    limit_price
                } else {
                    openbook.highest_placed_ask.max(limit_price)
                }
            }
            OpenbookV2Side::Bid => {
                // in base per quote units, to avoid a division in health
                let limit_price_inv = 1.0 / limit_price;
                openbook.highest_placed_bid_inv = if openbook.highest_placed_bid_inv == 0.0 {
                    limit_price_inv
                } else {
                    // the highest bid has the lowest _inv value
                    openbook.highest_placed_bid_inv.min(limit_price_inv)
                };
                openbook.lowest_placed_bid_inv = if openbook.lowest_placed_bid_inv == 0.0 {
                    limit_price_inv
                } else {
                    // lowest bid has max _inv value
                    openbook.lowest_placed_bid_inv.max(limit_price_inv)
                }
            }
        }
    }

    emit_stack(OpenbookV2OpenOrdersBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        market_index: openbook_market.market_index,
        base_token_index: openbook_market.base_token_index,
        quote_token_index: openbook_market.quote_token_index,
        base_total: after_oo.native_base_total(),
        base_free: after_oo.native_base_free(),
        quote_total: after_oo.native_quote_total(),
        quote_free: after_oo.native_quote_free(),
        referrer_rebates_accrued: after_oo.native_rebates(),
    });

    ctx.accounts.payer_vault.reload()?;
    let after_vault = ctx.accounts.payer_vault.amount;

    // Placing an order cannot increase vault balance
    require_gte!(before_vault, after_vault);

    let before_position_native;
    let vault_difference;
    {
        let mut payer_bank = ctx.accounts.payer_bank.load_mut()?;
        let mut receiver_bank = ctx.accounts.receiver_bank.load_mut()?;
        let (base_bank, quote_bank) = match order.side {
            OpenbookV2Side::Bid => (&mut receiver_bank, &mut payer_bank),
            OpenbookV2Side::Ask => (&mut payer_bank, &mut receiver_bank),
        };
        update_bank_potential_tokens(openbook, base_bank, quote_bank, &after_oo);

        // Track position before withdraw happens
        before_position_native = account
            .token_position_mut(payer_bank.token_index)?
            .0
            .native(&payer_bank);

        // Charge the difference in vault balance to the user's account
        vault_difference = {
            apply_vault_difference(
                ctx.accounts.account.key(),
                &mut account.borrow_mut(),
                SpotMarketIndex::OpenbookV2(openbook_market.market_index),
                &mut payer_bank,
                after_vault,
                before_vault,
            )?
        };
    }

    // Deposit limit check, receiver side:
    // Placing an order can always increase the receiver bank deposits on fill.
    {
        let receiver_bank = ctx.accounts.receiver_bank.load()?;
        receiver_bank
            .check_deposit_and_oo_limit()
            .with_context(|| std::format!("on {}", receiver_bank.name()))?;
    }

    // Payer bank safety checks like reduce-only, net borrows, vault-to-deposits ratio
    let withdrawn_from_vault = I80F48::from(before_vault - after_vault);
    let payer_bank = ctx.accounts.payer_bank.load()?;
    if withdrawn_from_vault > before_position_native {
        require_msg_typed!(
            !payer_bank.are_borrows_reduce_only(),
            MangoError::TokenInReduceOnlyMode,
            "the payer tokens cannot be borrowed"
        );
        payer_bank.enforce_max_utilization_on_borrow()?;
        payer_bank.check_net_borrows(payer_bank_oracle)?;

        // Deposit limit check, payer side:
        // The payer bank deposits could increase when cancelling the order later:
        // Imagine the account borrowing payer tokens to place the order, repaying the borrows
        // and then cancelling the order to create a deposit.
        //
        // However, if the account only decreases its deposits to place an order it can't
        // worsen the situation and should always go through, even if payer deposit limits are
        // already exceeded.
        payer_bank
            .check_deposit_and_oo_limit()
            .with_context(|| std::format!("on {}", payer_bank.name()))?;
    } else {
        payer_bank.enforce_borrows_lte_deposits()?;
    }

    // Limit order price bands: If the order ends up on the book, ensure
    // - a bid isn't too far below oracle
    // - an ask isn't too far above oracle
    // because placing orders that are guaranteed to never be hit can be bothersome:
    // For example placing a very large bid near zero would make the potential_base_tokens
    // value go through the roof, reducing available init margin for other users.
    let band_threshold = openbook_market.oracle_price_band();
    if new_order_on_book && band_threshold != f32::MAX {
        let (base_oracle, quote_oracle) = match order.side {
            OpenbookV2Side::Bid => (&receiver_bank_oracle, &payer_bank_oracle),
            OpenbookV2Side::Ask => (&payer_bank_oracle, &receiver_bank_oracle),
        };
        let base_oracle_f64 = base_oracle.to_num::<f64>();
        let quote_oracle_f64 = quote_oracle.to_num::<f64>();
        // this has the same units as base_oracle: USD per BASE; limit_price is in QUOTE per BASE
        let limit_price_in_dollar = limit_price * quote_oracle_f64;
        let band_factor = 1.0 + band_threshold as f64;
        match order.side {
            OpenbookV2Side::Bid => {
                require_msg_typed!(
                    limit_price_in_dollar * band_factor >= base_oracle_f64,
                    MangoError::SpotPriceBandExceeded,
                    "bid price {} must be larger than {} ({}% of oracle)",
                    limit_price,
                    base_oracle_f64 / (quote_oracle_f64 * band_factor),
                    (100.0 / band_factor) as u64,
                );
            }
            OpenbookV2Side::Ask => {
                require_msg_typed!(
                    limit_price_in_dollar <= base_oracle_f64 * band_factor,
                    MangoError::SpotPriceBandExceeded,
                    "ask price {} must be smaller than {} ({}% of oracle)",
                    limit_price,
                    base_oracle_f64 * band_factor / quote_oracle_f64,
                    (100.0 * band_factor) as u64,
                );
            }
        }
    }

    // Health cache updates for the changed account state
    let receiver_bank = ctx.accounts.receiver_bank.load()?;
    let payer_bank = ctx.accounts.payer_bank.load()?;
    // update scaled weights for receiver bank
    health_cache.adjust_token_balance(&receiver_bank, I80F48::ZERO)?;
    vault_difference.adjust_health_cache_token_balance(&mut health_cache, &payer_bank)?;
    let openbook_account = account.openbook_v2_orders(openbook_market.market_index)?;
    oo_difference.recompute_health_cache_openbook_v2_state(
        &mut health_cache,
        &openbook_account,
        &open_orders,
    )?;

    // Check the receiver's reduce only flag.
    //
    // Note that all orders on the book executing can still cause a net deposit. That's because
    // the total spot potential amount assumes all reserved amounts convert at the current
    // oracle price.
    //
    // This also requires that all spot oos that touch the receiver_token are avaliable in the
    // health cache. We make this a general requirement to avoid surprises.
    health_cache.check_has_all_spot_infos_for_token(&account.borrow(), receiver_token_index)?;
    if receiver_bank.are_deposits_reduce_only() {
        let balance = health_cache.token_info(receiver_token_index)?.balance_spot;
        let potential =
            health_cache.total_spot_potential(HealthType::Maint, receiver_token_index)?;
        require_msg_typed!(
            balance + potential < 1,
            MangoError::TokenInReduceOnlyMode,
            "receiver bank does not accept deposits"
        );
    }

    //
    // Health check
    //
    if let Some(pre_init_health) = pre_health_opt {
        account.check_health_post(&health_cache, pre_init_health)?;
    }

    Ok(())
}

/// Uses the changes in OpenOrders and vaults to adjust the user token position,
/// collect fees and optionally adjusts the HealthCache.
pub fn apply_settle_changes(
    group: &Group,
    account_pk: Pubkey,
    account: &mut MangoAccountRefMut,
    base_bank: &mut Bank,
    quote_bank: &mut Bank,
    openbook_market: &OpenbookV2Market,
    before_base_vault: u64,
    before_quote_vault: u64,
    before_oo: &OpenOrdersSlim,
    after_base_vault: u64,
    after_quote_vault: u64,
    after_oo: &OpenOrdersSlim,
    health_cache: Option<&mut HealthCache>,
    fees_to_dao: bool,
    quote_oracle: Option<&AccountInfo>,
    open_orders: &OpenOrdersAccount,
) -> Result<()> {
    let mut received_fees = 0;
    if fees_to_dao {
        // Example: rebates go from 100 -> 10. That means we credit 90 in fees.
        received_fees = before_oo
            .native_rebates()
            .saturating_sub(after_oo.native_rebates());
        quote_bank.collected_fees_native += I80F48::from(received_fees);

        // Credit the buyback_fees at the current value of the quote token.
        if let Some(quote_oracle_ai) = quote_oracle {
            let clock = Clock::get()?;
            let now_ts = clock.unix_timestamp.try_into().unwrap();

            let quote_oracle_ref = &AccountInfoRef::borrow(quote_oracle_ai)?;
            let quote_oracle_price = quote_bank.oracle_price(
                &OracleAccountInfos::from_reader(quote_oracle_ref),
                Some(clock.slot),
            )?;
            let quote_asset_price = quote_oracle_price.min(quote_bank.stable_price());
            account
                .fixed
                .expire_buyback_fees(now_ts, group.buyback_fees_expiry_interval);
            let fees_in_usd = I80F48::from(received_fees) * quote_asset_price;
            account
                .fixed
                .accrue_buyback_fees(fees_in_usd.clamp_to_u64());
        }
    }

    // Don't count the referrer rebate fees as part of the vault change that should be
    // credited to the user.
    let after_quote_vault_adjusted = after_quote_vault - received_fees;

    // Settle cannot decrease vault balances
    require_gte!(after_base_vault, before_base_vault);
    require_gte!(after_quote_vault_adjusted, before_quote_vault);

    // Credit the difference in vault balances to the user's account
    let base_difference = apply_vault_difference(
        account_pk,
        account,
        SpotMarketIndex::OpenbookV2(openbook_market.market_index),
        base_bank,
        after_base_vault,
        before_base_vault,
    )?;
    let quote_difference = apply_vault_difference(
        account_pk,
        account,
        SpotMarketIndex::OpenbookV2(openbook_market.market_index),
        quote_bank,
        after_quote_vault_adjusted,
        before_quote_vault,
    )?;

    // Tokens were moved from open orders into banks again: also update the tracking
    // for potential_serum_tokens on the banks.
    {
        let openbook_orders = account.openbook_v2_orders_mut(openbook_market.market_index)?;
        update_bank_potential_tokens(openbook_orders, base_bank, quote_bank, after_oo);
    }

    if let Some(health_cache) = health_cache {
        base_difference.adjust_health_cache_token_balance(health_cache, &base_bank)?;
        quote_difference.adjust_health_cache_token_balance(health_cache, &quote_bank)?;

        let serum_account = account.openbook_v2_orders(openbook_market.market_index)?;
        OODifference::new(&before_oo, &after_oo).recompute_health_cache_openbook_v2_state(
            health_cache,
            serum_account,
            open_orders,
        )?;
    }

    Ok(())
}

fn update_bank_potential_tokens(
    openbook_orders: &mut OpenbookV2Orders,
    base_bank: &mut Bank,
    quote_bank: &mut Bank,
    oo: &OpenOrdersSlim,
) {
    assert_eq!(openbook_orders.base_token_index, base_bank.token_index);
    assert_eq!(openbook_orders.quote_token_index, quote_bank.token_index);

    // Potential tokens are all tokens on the side, plus reserved on the other side
    // converted at favorable price. This creates an overestimation of the potential
    // base and quote tokens flowing out of this open orders account.
    let new_base = oo.native_base_total()
        + (oo.native_quote_reserved() as f64 * openbook_orders.lowest_placed_bid_inv) as u64;
    let new_quote = oo.native_quote_total()
        + (oo.native_base_reserved() as f64 * openbook_orders.highest_placed_ask) as u64;

    let old_base = openbook_orders.potential_base_tokens;
    let old_quote = openbook_orders.potential_quote_tokens;

    base_bank.update_potential_openbook_tokens(old_base, new_base);
    quote_bank.update_potential_openbook_tokens(old_quote, new_quote);

    openbook_orders.potential_base_tokens = new_base;
    openbook_orders.potential_quote_tokens = new_quote;
}

fn cpi_place_order(
    ctx: &OpenbookV2PlaceOrder,
    seeds: &[&[&[u8]]],
    order: &OpenbookV2Order,
    price_lots: i64,
    limit: u8,
) -> Result<Return<Option<u128>>> {
    let cpi_accounts = openbook_v2::cpi::accounts::PlaceOrder {
        signer: ctx.group.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        open_orders_admin: None,
        user_token_account: ctx.payer_vault.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        bids: ctx.bids.to_account_info(),
        asks: ctx.asks.to_account_info(),
        event_heap: ctx.event_heap.to_account_info(),
        market_vault: ctx.market_vault.to_account_info(),
        oracle_a: None, // we don't yet support markets with oracles
        oracle_b: None,
        token_program: ctx.token_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    let expiry_timestamp: u64 = if order.time_in_force > 0 {
        Clock::get()
            .unwrap()
            .unix_timestamp
            .saturating_add(order.time_in_force as i64)
            .try_into()
            .unwrap()
    } else {
        0
    };

    let order_type = match order.params {
        openbook_v2::state::OrderParams::Market => OpenbookV2OrderType::Market,
        openbook_v2::state::OrderParams::ImmediateOrCancel { price_lots } => {
            OpenbookV2OrderType::ImmediateOrCancel
        }
        openbook_v2::state::OrderParams::Fixed {
            price_lots,
            order_type,
        } => match order_type {
            openbook_v2::state::PostOrderType::Limit => OpenbookV2OrderType::Limit,
            openbook_v2::state::PostOrderType::PostOnly => OpenbookV2OrderType::PostOnly,
            openbook_v2::state::PostOrderType::PostOnlySlide => OpenbookV2OrderType::PostOnlySlide,
        },
        openbook_v2::state::OrderParams::OraclePegged {
            price_offset_lots,
            order_type,
            peg_limit,
        } => todo!(),
    };

    let args = openbook_v2::PlaceOrderArgs {
        side: order.side,
        price_lots,
        max_base_lots: order.max_base_lots,
        max_quote_lots_including_fees: order.max_quote_lots_including_fees,
        client_order_id: order.client_order_id,
        order_type,
        expiry_timestamp,
        self_trade_behavior: order.self_trade_behavior,
        limit,
    };

    msg!("args {:?}", args);
    openbook_v2::cpi::place_order(cpi_ctx, args)
}
