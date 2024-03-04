use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::i80f48::ClampToInt;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{emit_stack, Serum3OpenOrdersBalanceLogV2, TokenBalanceLog};
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
    require_v2: bool,
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
    let receiver_bank_ai;
    let receiver_bank_oracle;
    let receiver_bank_reduce_only;
    {
        // The token position already exists, but we need the active_index.
        let (_, _, active_index) = account.ensure_token_position(receiver_token_index)?;
        let group_key = ctx.accounts.group.key();
        let (receiver_bank, oracle) =
            retriever.bank_and_oracle(&group_key, active_index, receiver_token_index)?;
        receiver_bank_oracle = oracle;
        receiver_bank_reduce_only = receiver_bank.are_deposits_reduce_only();

        // The fixed_order account retriever can't give us mut references, so use the above
        // call to .bank_and_oracle() as validation and then copy out the matching AccountInfo.
        receiver_bank_ai = ctx.remaining_accounts[active_index].clone();
        // Double-check that we got the right account
        let receiver_bank2 = receiver_bank_ai.load::<Bank>()?;
        assert_eq!(receiver_bank2.group, group_key);
        assert_eq!(receiver_bank2.token_index, receiver_token_index);
    }

    drop(retriever);

    //
    // Instruction version checking #4
    //
    let is_v2_instruction;
    {
        let group = ctx.accounts.group.load()?;
        let v1_available = group.is_ix_enabled(IxGate::Serum3PlaceOrder);
        let v2_available = group.is_ix_enabled(IxGate::Serum3PlaceOrderV2);
        is_v2_instruction =
            require_v2 || !v1_available || (receiver_bank_ai.is_writable && v2_available);
        if is_v2_instruction {
            require!(v2_available, MangoError::IxIsDisabled);
            require_msg_typed!(
                receiver_bank_ai.is_writable,
                MangoError::HealthAccountBankNotWritable,
                "the receiver bank (token index {}) in the health account list must be writable",
                receiver_token_index
            );
        } else {
            require!(v1_available, MangoError::IxIsDisabled);
        }
    }

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
        serum.lowest_placed_bid_inv = 0.0;
    }
    if !before_had_asks {
        serum.lowest_placed_ask = 0.0;
        serum.highest_placed_ask = 0.0;
    }
    // in the normal quote per base units
    let limit_price = limit_price_lots as f64 * quote_lot_size as f64 / base_lot_size as f64;

    let new_order_on_book = after_oo_free_slots != before_oo_free_slots;
    if new_order_on_book {
        match side {
            Serum3Side::Ask => {
                serum.lowest_placed_ask = if serum.lowest_placed_ask == 0.0 {
                    limit_price
                } else {
                    serum.lowest_placed_ask.min(limit_price)
                };
                serum.highest_placed_ask = if serum.highest_placed_ask == 0.0 {
                    limit_price
                } else {
                    serum.highest_placed_ask.max(limit_price)
                }
            }
            Serum3Side::Bid => {
                // in base per quote units, to avoid a division in health
                let limit_price_inv = 1.0 / limit_price;
                serum.highest_placed_bid_inv = if serum.highest_placed_bid_inv == 0.0 {
                    limit_price_inv
                } else {
                    // the highest bid has the lowest _inv value
                    serum.highest_placed_bid_inv.min(limit_price_inv)
                };
                serum.lowest_placed_bid_inv = if serum.lowest_placed_bid_inv == 0.0 {
                    limit_price_inv
                } else {
                    // lowest bid has max _inv value
                    serum.lowest_placed_bid_inv.max(limit_price_inv)
                }
            }
        }
    }

    emit_stack(Serum3OpenOrdersBalanceLogV2 {
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

    // Update the potential token tracking in banks
    // (for init weight scaling, deposit limit checks)
    if is_v2_instruction {
        let mut receiver_bank = receiver_bank_ai.load_mut::<Bank>()?;
        let (base_bank, quote_bank) = match side {
            Serum3Side::Bid => (&mut receiver_bank, &mut payer_bank),
            Serum3Side::Ask => (&mut payer_bank, &mut receiver_bank),
        };
        update_bank_potential_tokens(serum, base_bank, quote_bank, &after_oo);
    } else {
        update_bank_potential_tokens_payer_only(serum, &mut payer_bank, &after_oo);
    }

    // Track position before withdraw happens
    let before_position_native = account
        .token_position_mut(payer_bank.token_index)?
        .0
        .native(&payer_bank);

    // Charge the difference in vault balance to the user's account
    // (must be done before limit checks like deposit limit)
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

    // Deposit limit check, receiver side:
    // Placing an order can always increase the receiver bank deposits on fill.
    {
        let receiver_bank = receiver_bank_ai.load::<Bank>()?;
        receiver_bank
            .check_deposit_and_oo_limit()
            .with_context(|| std::format!("on {}", receiver_bank.name()))?;
    }

    // Payer bank safety checks like reduce-only, net borrows, vault-to-deposits ratio
    let payer_oracle_ref = &AccountInfoRef::borrow(&ctx.accounts.payer_oracle)?;
    let payer_bank_oracle =
        payer_bank.oracle_price(&OracleAccountInfos::from_reader(payer_oracle_ref), None)?;
    let withdrawn_from_vault = I80F48::from(before_vault - after_vault);
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
    let band_threshold = serum_market.oracle_price_band();
    if new_order_on_book && band_threshold != f32::MAX {
        let (base_oracle, quote_oracle) = match side {
            Serum3Side::Bid => (&receiver_bank_oracle, &payer_bank_oracle),
            Serum3Side::Ask => (&payer_bank_oracle, &receiver_bank_oracle),
        };
        let base_oracle_f64 = base_oracle.to_num::<f64>();
        let quote_oracle_f64 = quote_oracle.to_num::<f64>();
        // this has the same units as base_oracle: USD per BASE; limit_price is in QUOTE per BASE
        let limit_price_in_dollar = limit_price * quote_oracle_f64;
        let band_factor = 1.0 + band_threshold as f64;
        match side {
            Serum3Side::Bid => {
                require_msg_typed!(
                    limit_price_in_dollar * band_factor >= base_oracle_f64,
                    MangoError::Serum3PriceBandExceeded,
                    "bid price {} must be larger than {} ({}% of oracle)",
                    limit_price,
                    base_oracle_f64 / (quote_oracle_f64 * band_factor),
                    (100.0 / band_factor) as u64,
                );
            }
            Serum3Side::Ask => {
                require_msg_typed!(
                    limit_price_in_dollar <= base_oracle_f64 * band_factor,
                    MangoError::Serum3PriceBandExceeded,
                    "ask price {} must be smaller than {} ({}% of oracle)",
                    limit_price,
                    base_oracle_f64 * band_factor / quote_oracle_f64,
                    (100.0 * band_factor) as u64,
                );
            }
        }
    }

    // Health cache updates for the changed account state
    let receiver_bank = receiver_bank_ai.load::<Bank>()?;
    // update scaled weights for receiver bank
    health_cache.adjust_token_balance(&receiver_bank, I80F48::ZERO)?;
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
    // amount of tokens transfered to serum3 reserved that were borrowed
    let new_borrows = native_change
        .max(native_after)
        .min(I80F48::ZERO)
        .abs()
        .to_num::<u64>();

    let indexed_position = position.indexed_position;
    let market = account.serum3_orders_mut(serum_market_index).unwrap();
    let borrows_without_fee;
    if bank.token_index == market.base_token_index {
        borrows_without_fee = &mut market.base_borrows_without_fee;
    } else if bank.token_index == market.quote_token_index {
        borrows_without_fee = &mut market.quote_borrows_without_fee;
    } else {
        return Err(error_msg!(
            "assert failed: apply_vault_difference called with bad token index"
        ));
    };

    // Only for place: Add to potential borrow amount
    *borrows_without_fee += new_borrows;

    // Only for settle/liq_force_cancel: Reduce the potential borrow amounts
    if needed_change > 0 {
        *borrows_without_fee = (*borrows_without_fee).saturating_sub(needed_change.to_num::<u64>());
    }

    emit_stack(TokenBalanceLog {
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

    // Tokens were moved from open orders into banks again: also update the tracking
    // for potential_serum_tokens on the banks.
    {
        let serum_orders = account.serum3_orders_mut(serum_market.market_index)?;
        update_bank_potential_tokens(serum_orders, base_bank, quote_bank, after_oo);
    }

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

fn update_bank_potential_tokens_payer_only(
    serum_orders: &mut Serum3Orders,
    payer_bank: &mut Bank,
    oo: &OpenOrdersSlim,
) {
    // Do the tracking for the avaliable bank
    if serum_orders.base_token_index == payer_bank.token_index {
        let new_base = oo.native_base_total()
            + (oo.native_quote_reserved() as f64 * serum_orders.lowest_placed_bid_inv) as u64;
        let old_base = serum_orders.potential_base_tokens;

        payer_bank.update_potential_serum_tokens(old_base, new_base);
        serum_orders.potential_base_tokens = new_base;
    } else {
        assert_eq!(serum_orders.quote_token_index, payer_bank.token_index);

        let new_quote = oo.native_quote_total()
            + (oo.native_base_reserved() as f64 * serum_orders.highest_placed_ask) as u64;
        let old_quote = serum_orders.potential_quote_tokens;

        payer_bank.update_potential_serum_tokens(old_quote, new_quote);
        serum_orders.potential_quote_tokens = new_quote;
    }
}

fn update_bank_potential_tokens(
    serum_orders: &mut Serum3Orders,
    base_bank: &mut Bank,
    quote_bank: &mut Bank,
    oo: &OpenOrdersSlim,
) {
    assert_eq!(serum_orders.base_token_index, base_bank.token_index);
    assert_eq!(serum_orders.quote_token_index, quote_bank.token_index);

    // Potential tokens are all tokens on the side, plus reserved on the other side
    // converted at favorable price. This creates an overestimation of the potential
    // base and quote tokens flowing out of this open orders account.
    let new_base = oo.native_base_total()
        + (oo.native_quote_reserved() as f64 * serum_orders.lowest_placed_bid_inv) as u64;
    let new_quote = oo.native_quote_total()
        + (oo.native_base_reserved() as f64 * serum_orders.highest_placed_ask) as u64;

    let old_base = serum_orders.potential_base_tokens;
    let old_quote = serum_orders.potential_quote_tokens;

    base_bank.update_potential_serum_tokens(old_base, new_base);
    quote_bank.update_potential_serum_tokens(old_quote, new_quote);

    serum_orders.potential_base_tokens = new_base;
    serum_orders.potential_quote_tokens = new_quote;
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
