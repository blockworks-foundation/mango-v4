use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{Serum3OpenOrdersBalanceLogV2, TokenBalanceLog};
use crate::serum3_cpi::{
    load_market_state, load_open_orders_ref, OpenOrdersAmounts, OpenOrdersSlim,
};
use anchor_lang::prelude::*;

use fixed::types::I80F48;
use serum_dex::instruction::NewOrderInstructionV3;

#[allow(clippy::too_many_arguments)]
pub fn serum3_place_order(
    ctx: Context<Serum3PlaceOrder>,
    side: Serum3Side,
    limit_price_lots: u64,
    max_base_qty: u64,
    max_native_quote_qty_including_fees: u64,
    self_trade_behavior: Serum3SelfTradeBehavior,
    order_type: Serum3OrderType,
    client_order_id: u64,
    limit: u16,
) -> Result<()> {
    // Also required by serum3's place order
    require_gt!(limit_price_lots, 0);

    let serum_market = ctx.accounts.serum_market.load()?;
    require!(
        !serum_market.is_reduce_only(),
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
            account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
            MangoError::SomeError
        );

        // Validate open_orders #2
        require!(
            account
                .serum3_orders(serum_market.market_index)?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );

        // Validate bank and vault #3
        let payer_bank = ctx.accounts.payer_bank.load()?;
        require_keys_eq!(payer_bank.vault, ctx.accounts.payer_vault.key());
        let payer_token_index = match side {
            Serum3Side::Bid => serum_market.quote_token_index,
            Serum3Side::Ask => serum_market.base_token_index,
        };
        require_eq!(payer_bank.token_index, payer_token_index);

        receiver_token_index = match side {
            Serum3Side::Bid => serum_market.base_token_index,
            Serum3Side::Ask => serum_market.quote_token_index,
        };
    }

    //
    // Pre-health computation
    //
    let mut account = ctx.accounts.account.load_full_mut()?;
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let mut health_cache =
        new_health_cache(&account.borrow(), &retriever).context("pre-withdraw init health")?;
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

    let before_vault = ctx.accounts.payer_vault.amount;

    let before_oo_free_slots;
    let before_had_bids;
    let before_had_asks;
    let before_oo = {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        before_oo_free_slots = open_orders.free_slot_bits;
        before_had_bids = (!open_orders.free_slot_bits & open_orders.is_bid_bits) != 0;
        before_had_asks = (!open_orders.free_slot_bits & !open_orders.is_bid_bits) != 0;
        OpenOrdersSlim::from_oo(&open_orders)
    };

    // Provide a readable error message in case the vault doesn't have enough tokens
    let base_lot_size;
    let quote_lot_size;
    {
        let market_state = load_market_state(
            &ctx.accounts.serum_market_external,
            &ctx.accounts.serum_program.key(),
        )?;
        base_lot_size = market_state.coin_lot_size;
        quote_lot_size = market_state.pc_lot_size;

        let needed_amount = match side {
            Serum3Side::Ask => {
                (max_base_qty * base_lot_size).saturating_sub(before_oo.native_base_free())
            }
            Serum3Side::Bid => {
                max_native_quote_qty_including_fees.saturating_sub(before_oo.native_quote_free())
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
    // Apply the order to serum
    //
    let order = serum_dex::instruction::NewOrderInstructionV3 {
        side: u8::try_from(side).unwrap().try_into().unwrap(),
        limit_price: limit_price_lots.try_into().unwrap(),
        max_coin_qty: max_base_qty.try_into().unwrap(),
        max_native_pc_qty_including_fees: max_native_quote_qty_including_fees.try_into().unwrap(),
        self_trade_behavior: u8::try_from(self_trade_behavior)
            .unwrap()
            .try_into()
            .unwrap(),
        order_type: u8::try_from(order_type).unwrap().try_into().unwrap(),
        client_order_id,
        limit,
        max_ts: i64::MAX,
    };
    cpi_place_order(ctx.accounts, order)?;

    //
    // After-order tracking
    //
    let after_oo_free_slots;
    let after_oo = {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        after_oo_free_slots = open_orders.free_slot_bits;
        OpenOrdersSlim::from_oo(&open_orders)
    };
    let oo_difference = OODifference::new(&before_oo, &after_oo);

    //
    // Track the highest bid and lowest ask, to be able to evaluate worst-case health even
    // when they cross the oracle
    //
    let serum = account.serum3_orders_mut(serum_market.market_index)?;
    if !before_had_bids {
        // The 0 state means uninitialized/no value
        serum.highest_placed_bid_inv = 0.0;
    }
    if !before_had_asks {
        serum.lowest_placed_ask = 0.0;
    }
    let new_order_on_book = after_oo_free_slots != before_oo_free_slots;
    if new_order_on_book {
        match side {
            Serum3Side::Ask => {
                // in the normal quote per base units
                let limit_price =
                    limit_price_lots as f64 * quote_lot_size as f64 / base_lot_size as f64;
                serum.lowest_placed_ask = if serum.lowest_placed_ask == 0.0 {
                    limit_price
                } else {
                    serum.lowest_placed_ask.min(limit_price)
                };
            }
            Serum3Side::Bid => {
                // in base per quote units, to avoid a division in health
                let limit_price_inv =
                    base_lot_size as f64 / (limit_price_lots as f64 * quote_lot_size as f64);
                serum.highest_placed_bid_inv = if serum.highest_placed_bid_inv == 0.0 {
                    limit_price_inv
                } else {
                    // the highest bid has the lowest _inv value
                    serum.highest_placed_bid_inv.min(limit_price_inv)
                };
            }
        }
    }

    emit!(Serum3OpenOrdersBalanceLogV2 {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        market_index: serum_market.market_index,
        base_token_index: serum_market.base_token_index,
        quote_token_index: serum_market.quote_token_index,
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

    let mut payer_bank = ctx.accounts.payer_bank.load_mut()?;

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
            serum_market.market_index,
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
            payer_bank.oracle_price(&AccountInfoRef::borrow(&ctx.accounts.payer_oracle)?, None)?;
        payer_bank.enforce_min_vault_to_deposits_ratio((*ctx.accounts.payer_vault).as_ref())?;
        payer_bank.check_net_borrows(oracle_price)?;
    }

    vault_difference.adjust_health_cache_token_balance(&mut health_cache, &payer_bank)?;

    let serum_account = account.serum3_orders(serum_market.market_index)?;
    oo_difference.recompute_health_cache_serum3_state(
        &mut health_cache,
        &serum_account,
        &after_oo,
    )?;

    // Check the receiver's reduce only flag.
    //
    // Note that all orders on the book executing can still cause a net deposit. That's because
    // the total serum3 potential amount assumes all reserved amounts convert at the current
    // oracle price.
    if receiver_bank_reduce_only {
        let balance = health_cache.token_info(receiver_token_index)?.balance_spot;
        let potential =
            health_cache.total_serum3_potential(HealthType::Maint, receiver_token_index)?;
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

pub struct OODifference {
    free_base_change: I80F48,
    free_quote_change: I80F48,
}

impl OODifference {
    pub fn new(before_oo: &OpenOrdersSlim, after_oo: &OpenOrdersSlim) -> Self {
        Self {
            free_base_change: I80F48::from(after_oo.native_base_free())
                - I80F48::from(before_oo.native_base_free()),
            free_quote_change: I80F48::from(after_oo.native_quote_free())
                - I80F48::from(before_oo.native_quote_free()),
        }
    }

    pub fn recompute_health_cache_serum3_state(
        &self,
        health_cache: &mut HealthCache,
        serum_account: &Serum3Orders,
        open_orders: &OpenOrdersSlim,
    ) -> Result<()> {
        health_cache.recompute_serum3_info(
            serum_account,
            open_orders,
            self.free_base_change,
            self.free_quote_change,
        )
    }
}

pub struct VaultDifference {
    token_index: TokenIndex,
    native_change: I80F48,
}

impl VaultDifference {
    pub fn adjust_health_cache_token_balance(
        &self,
        health_cache: &mut HealthCache,
        bank: &Bank,
    ) -> Result<()> {
        assert_eq!(bank.token_index, self.token_index);
        health_cache.adjust_token_balance(bank, self.native_change)?;
        Ok(())
    }
}

/// Called in apply_settle_changes() and place_order to adjust token positions after
/// changing the vault balances
/// Also logs changes to token balances
fn apply_vault_difference(
    account_pk: Pubkey,
    account: &mut MangoAccountRefMut,
    serum_market_index: Serum3MarketIndex,
    bank: &mut Bank,
    vault_after: u64,
    vault_before: u64,
) -> Result<VaultDifference> {
    let needed_change = I80F48::from(vault_after) - I80F48::from(vault_before);

    let (position, _) = account.token_position_mut(bank.token_index)?;
    let native_before = position.native(bank);
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();
    if needed_change >= 0 {
        bank.deposit(position, needed_change, now_ts)?;
    } else {
        bank.withdraw_without_fee(position, -needed_change, now_ts)?;
    }
    let native_after = position.native(bank);
    let native_change = native_after - native_before;
    let new_borrows = native_change
        .max(native_after)
        .min(I80F48::ZERO)
        .abs()
        .to_num::<u64>();

    let indexed_position = position.indexed_position;
    let market = account.serum3_orders_mut(serum_market_index).unwrap();
    let borrows_without_fee = if bank.token_index == market.base_token_index {
        &mut market.base_borrows_without_fee
    } else if bank.token_index == market.quote_token_index {
        &mut market.quote_borrows_without_fee
    } else {
        return Err(error_msg!(
            "assert failed: apply_vault_difference called with bad token index"
        ));
    };

    // Only for place: Add to potential borrow amount
    let old_value = *borrows_without_fee;
    *borrows_without_fee = old_value + new_borrows;

    // Only for settle/liq_force_cancel: Reduce the potential borrow amounts
    if needed_change > 0 {
        *borrows_without_fee = (*borrows_without_fee).saturating_sub(needed_change.to_num::<u64>());
    }

    emit!(TokenBalanceLog {
        mango_group: bank.group,
        mango_account: account_pk,
        token_index: bank.token_index,
        indexed_position: indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    Ok(VaultDifference {
        token_index: bank.token_index,
        native_change,
    })
}

/// Uses the changes in OpenOrders and vaults to adjust the user token position,
/// collect fees and optionally adjusts the HealthCache.
pub fn apply_settle_changes(
    group: &Group,
    account_pk: Pubkey,
    account: &mut MangoAccountRefMut,
    base_bank: &mut Bank,
    quote_bank: &mut Bank,
    serum_market: &Serum3Market,
    before_base_vault: u64,
    before_quote_vault: u64,
    before_oo: &OpenOrdersSlim,
    after_base_vault: u64,
    after_quote_vault: u64,
    after_oo: &OpenOrdersSlim,
    health_cache: Option<&mut HealthCache>,
    fees_to_dao: bool,
    quote_oracle: Option<&AccountInfo>,
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
        serum_market.market_index,
        base_bank,
        after_base_vault,
        before_base_vault,
    )?;
    let quote_difference = apply_vault_difference(
        account_pk,
        account,
        serum_market.market_index,
        quote_bank,
        after_quote_vault_adjusted,
        before_quote_vault,
    )?;

    if let Some(health_cache) = health_cache {
        base_difference.adjust_health_cache_token_balance(health_cache, &base_bank)?;
        quote_difference.adjust_health_cache_token_balance(health_cache, &quote_bank)?;

        let serum_account = account.serum3_orders(serum_market.market_index)?;
        OODifference::new(&before_oo, &after_oo).recompute_health_cache_serum3_state(
            health_cache,
            serum_account,
            after_oo,
        )?;
    }

    Ok(())
}

fn cpi_place_order(ctx: &Serum3PlaceOrder, order: NewOrderInstructionV3) -> Result<()> {
    use crate::serum3_cpi;

    let group = ctx.group.load()?;
    serum3_cpi::PlaceOrder {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        request_queue: ctx.market_request_queue.to_account_info(),
        event_queue: ctx.market_event_queue.to_account_info(),
        bids: ctx.market_bids.to_account_info(),
        asks: ctx.market_asks.to_account_info(),
        base_vault: ctx.market_base_vault.to_account_info(),
        quote_vault: ctx.market_quote_vault.to_account_info(),
        token_program: ctx.token_program.to_account_info(),

        open_orders: ctx.open_orders.to_account_info(),
        order_payer_token_account: ctx.payer_vault.to_account_info(),
        user_authority: ctx.group.to_account_info(),
    }
    .call(&group, order)
}
