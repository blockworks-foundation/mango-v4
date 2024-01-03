use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::instructions::{apply_vault_difference, OODifference};
use crate::serum3_cpi::{OpenOrdersAmounts, OpenOrdersSlim};
use crate::state::*;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use openbook_v2::cpi::Return;
use openbook_v2::state::OpenOrdersAccount;
use openbook_v2::state::{
    OpenOrder, Order as OpenbookV2Order, PlaceOrderType as OpenbookV2OrderType,
    Side as OpenbookV2Side, MAX_OPEN_ORDERS,
};

use crate::accounts_ix::*;

pub fn openbook_v2_place_order(
    ctx: Context<OpenbookV2PlaceOrder>,
    order: OpenbookV2Order,
    limit: u16,
) -> Result<()> {
    let openbook_market = ctx.accounts.openbook_v2_market.load()?;
    require!(
        !openbook_market.is_reduce_only(),
        MangoError::MarketInReduceOnlyMode
    );

    //
    // Validation
    //
    let receiver_token_index;
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

        // Validate bank and vault #3
        let payer_bank = ctx.accounts.bank.load()?;
        require_keys_eq!(payer_bank.vault, ctx.accounts.vault.key());
        let payer_token_index = match order.side {
            OpenbookV2Side::Bid => openbook_market.quote_token_index,
            OpenbookV2Side::Ask => openbook_market.base_token_index,
        };
        require_eq!(payer_bank.token_index, payer_token_index);

        receiver_token_index = match order.side {
            OpenbookV2Side::Bid => openbook_market.base_token_index,
            OpenbookV2Side::Ask => openbook_market.quote_token_index,
        };
    }

    //
    // Pre-health computation
    //
    let mut account = ctx.accounts.account.load_full_mut()?;
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
    let mut health_cache = new_health_cache(&account.borrow(), &retriever, now_ts)
        .context("pre-withdraw init health")?;
    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let pre_init_health = account.check_health_pre(&health_cache)?;
        Some(pre_init_health)
    } else {
        None
    };

    // Check if the bank for the token whose balance is increased is in reduce-only mode
    let receiver_bank_reduce_only = {
        // The token position already exists, but we need the active_index.
        let (_, _, active_index) = account.ensure_token_position(receiver_token_index)?;
        let group_key = ctx.accounts.group.key();
        let receiver_bank = retriever
            .bank_and_oracle(&group_key, active_index, receiver_token_index)?
            .0;
        receiver_bank.are_deposits_reduce_only()
    };

    drop(retriever);

    //
    // Before-order tracking
    //

    let before_vault = ctx.accounts.vault.amount;

    let before_oo_free_slots;
    let before_had_bids;
    let before_had_asks;
    let before_oo = {
        let open_orders = ctx.accounts.open_orders.load()?;
        before_oo_free_slots = MAX_OPEN_ORDERS
            - open_orders
                .all_orders_in_use()
                .collect::<Vec<&OpenOrder>>()
                .len();
        before_had_bids = open_orders.position.bids_base_lots != 0;
        before_had_asks = open_orders.position.asks_base_lots != 0;
        OpenOrdersSlim::from_oo_v2(&open_orders)
    };

    // Provide a readable error message in case the vault doesn't have enough tokens
    let base_lot_size;
    let quote_lot_size;
    {
        let openbook_market_external = ctx.accounts.openbook_v2_market_external.load()?;
        base_lot_size = openbook_market_external.base_lot_size;
        quote_lot_size = openbook_market_external.quote_lot_size;

        // todo-pan: why i64? hope the cast doesnt fuck anything up
        let needed_amount = match order.side {
            OpenbookV2Side::Ask => {
                (order.max_base_lots as u64 * base_lot_size as u64).saturating_sub(before_oo.native_base_free())
            }
            OpenbookV2Side::Bid => {
                (order.max_quote_lots_including_fees as u64).saturating_sub(before_oo.native_quote_free())
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
    }

    //
    // CPI to place order
    //
    let account_seeds = mango_account_seeds!(account.fixed);
    cpi_place_order(ctx.accounts, &[account_seeds], &order, limit.try_into().unwrap())?;

    //
    // After-order tracking
    //
    let after_oo_free_slots;
    let after_oo = {
        let open_orders = ctx.accounts.open_orders.load()?;
        after_oo_free_slots = MAX_OPEN_ORDERS
            - open_orders
                .all_orders_in_use()
                .collect::<Vec<&OpenOrder>>()
                .len();
        OpenOrdersSlim::from_oo_v2(&open_orders)
    };
    let oo_difference = OODifference::new(&before_oo, &after_oo);

    //
    // Track the highest bid and lowest ask, to be able to evaluate worst-case health even
    // when they cross the oracle
    //
    let openbook = account.openbook_v2_orders_mut(openbook_market.market_index)?;
    if !before_had_bids {
        // The 0 state means uninitialized/no value
        openbook.highest_placed_bid_inv = 0.0;
    }
    if !before_had_asks {
        openbook.lowest_placed_ask = 0.0;
    }
    let new_order_on_book = after_oo_free_slots != before_oo_free_slots;
    if new_order_on_book {
        match order.side {
            OpenbookV2Side::Ask => {
                // in the normal quote per base units
                let limit_price = order.max_quote_lots_including_fees as f64 * quote_lot_size as f64
                    / base_lot_size as f64;
                openbook.lowest_placed_ask = if openbook.lowest_placed_ask == 0.0 {
                    limit_price
                } else {
                    openbook.lowest_placed_ask.min(limit_price)
                };
            }
            OpenbookV2Side::Bid => {
                // in base per quote units, to avoid a division in health
                let limit_price_inv = base_lot_size as f64
                    / (order.max_quote_lots_including_fees as f64 * quote_lot_size as f64);
                openbook.highest_placed_bid_inv = if openbook.highest_placed_bid_inv == 0.0 {
                    limit_price_inv
                } else {
                    // the highest bid has the lowest _inv value
                    openbook.highest_placed_bid_inv.min(limit_price_inv)
                };
            }
        }
    }

    // todo-pan: add logs
    // emit!(OpenbookV2OpenOrdersBalanceLog {
    //     mango_group: ctx.accounts.group.key(),
    //     mango_account: ctx.accounts.account.key(),
    //     market_index: openbook_market.market_index,
    //     base_token_index: openbook_market.base_token_index,
    //     quote_token_index: openbook_market.quote_token_index,
    //     base_total: after_oo.native_base_total(),
    //     base_free: after_oo.native_base_free(),
    //     quote_total: after_oo.native_quote_total(),
    //     quote_free: after_oo.native_quote_free(),
    //     referrer_rebates_accrued: after_oo.native_rebates(),
    // });

    ctx.accounts.vault.reload()?;
    let after_vault = ctx.accounts.vault.amount;

    // Placing an order cannot increase vault balance
    require_gte!(before_vault, after_vault);

    let mut payer_bank = ctx.accounts.bank.load_mut()?;

    // Enforce min vault to deposits ratio
    let withdrawn_from_vault = I80F48::from(before_vault - after_vault);
    let position_native = account
        .token_position_mut(payer_bank.token_index)?
        .0
        .native(&payer_bank);

    // Charge the difference in vault balance to the user's account
    let vault_difference = {
        apply_vault_difference(
            ctx.accounts.account.key(),
            &mut account.borrow_mut(),
            openbook_market.market_index,
            &mut payer_bank,
            after_vault,
            before_vault,
        )?
    };

    if withdrawn_from_vault > position_native {
        require_msg_typed!(
            !payer_bank.are_borrows_reduce_only(),
            MangoError::TokenInReduceOnlyMode,
            "the payer tokens cannot be borrowed"
        );
        let oracle_price =
            payer_bank.oracle_price(&AccountInfoRef::borrow(&ctx.accounts.oracle)?, None)?;
        payer_bank.enforce_min_vault_to_deposits_ratio((*ctx.accounts.vault).as_ref())?;
        payer_bank.check_net_borrows(oracle_price)?;
    }

    vault_difference.adjust_health_cache_token_balance(&mut health_cache, &payer_bank)?;

    let openbook_orders = account.openbook_v2_orders(openbook_market.market_index)?;
    let open_orders_account = ctx.accounts.open_orders.load()?;
    oo_difference.recompute_health_cache_openbook_v2_state(
        &mut health_cache,
        &openbook_orders,
        &*open_orders_account, // todo-pan : absolutely disgusting
    )?;

    // Check the receiver's reduce only flag.
    //
    // Note that all orders on the book executing can still cause a net deposit. That's because
    // the total serum3 potential amount assumes all reserved amounts convert at the current
    // oracle price.
    if receiver_bank_reduce_only {
        let balance = health_cache.token_info(receiver_token_index)?.balance_spot;
        let potential =
            health_cache.total_spot_potential(HealthType::Maint, receiver_token_index)?; // todo-pan: split potential into serum and openbook
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
pub fn apply_settle_changes_v2(
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

            let quote_oracle_price = quote_bank
                .oracle_price(&AccountInfoRef::borrow(quote_oracle_ai)?, Some(clock.slot))?;
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
        openbook_market.market_index,
        base_bank,
        after_base_vault,
        before_base_vault,
    )?;
    let quote_difference = apply_vault_difference(
        account_pk,
        account,
        openbook_market.market_index,
        quote_bank,
        after_quote_vault_adjusted,
        before_quote_vault,
    )?;

    // Tokens were moved from open orders into banks again: also update the tracking
    // for deposits_in_openbook on the banks.
    {
        let openbook_orders = account.serum3_orders_mut(openbook_market.market_index)?;

        let after_base_reserved = after_oo.native_base_reserved();
        if after_base_reserved < openbook_orders.base_deposits_reserved {
            let diff = openbook_orders.base_deposits_reserved - after_base_reserved;
            openbook_orders.base_deposits_reserved = after_base_reserved;
            let diff_signed: i64 = diff.try_into().unwrap();
            base_bank.deposits_in_openbook -= diff_signed;
        }

        let after_quote_reserved = after_oo.native_quote_reserved();
        if after_quote_reserved < openbook_orders.quote_deposits_reserved {
            let diff = openbook_orders.quote_deposits_reserved - after_quote_reserved;
            openbook_orders.quote_deposits_reserved = after_quote_reserved;
            let diff_signed: i64 = diff.try_into().unwrap();
            quote_bank.deposits_in_openbook -= diff_signed;
        }
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

fn cpi_place_order(
    ctx: &OpenbookV2PlaceOrder,
    seeds: &[&[&[u8]]],
    order: &OpenbookV2Order,
    limit: u8,
) -> Result<Return<Option<u128>>> {
    let cpi_accounts = openbook_v2::cpi::accounts::PlaceOrder {
        signer: ctx.account.to_account_info(),
        open_orders_account: ctx.open_orders.to_account_info(),
        open_orders_admin: None,
        user_token_account: ctx.vault.to_account_info(),
        market: ctx.openbook_v2_market_external.to_account_info(),
        bids: ctx.bids.to_account_info(),
        asks: ctx.asks.to_account_info(),
        event_heap: ctx.event_heap.to_account_info(),
        market_vault: ctx.market_vault.to_account_info(),
        oracle_a: None, // todo-pan: how do oracle work
        oracle_b: None,
        token_program: ctx.token_program.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.openbook_v2_program.to_account_info(),
        cpi_accounts,
        seeds,
    );

    let price_lots = {
        let market = ctx.openbook_v2_market_external.load()?;
        market.native_price_to_lot(I80F48::from(1000)).unwrap()
    };

    let expiry_timestamp: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();

    let args = openbook_v2::PlaceOrderArgs {
        side: order.side,
        price_lots,
        max_base_lots: order.max_base_lots,
        max_quote_lots_including_fees: order.max_quote_lots_including_fees,
        client_order_id: order.client_order_id,
        order_type: OpenbookV2OrderType::Limit,
        expiry_timestamp,
        self_trade_behavior: order.self_trade_behavior,
        limit,
    };
    openbook_v2::cpi::place_order(cpi_ctx, args)
}
