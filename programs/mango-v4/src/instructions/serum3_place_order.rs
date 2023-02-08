use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::health::*;
use crate::state::*;

use crate::logs::{Serum3OpenOrdersBalanceLogV2, TokenBalanceLog};
use crate::serum3_cpi::{load_market_state, load_open_orders_ref};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use checked_math as cm;
use fixed::types::I80F48;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use serum_dex::instruction::NewOrderInstructionV3;
use serum_dex::state::OpenOrders;

/// For loan origination fees bookkeeping purposes
#[derive(Debug)]
pub struct OpenOrdersSlim {
    native_coin_free: u64,
    native_coin_total: u64,
    native_pc_free: u64,
    native_pc_total: u64,
    referrer_rebates_accrued: u64,
}
impl OpenOrdersSlim {
    pub fn from_oo(oo: &OpenOrders) -> Self {
        Self {
            native_coin_free: oo.native_coin_free,
            native_coin_total: oo.native_coin_total,
            native_pc_free: oo.native_pc_free,
            native_pc_total: oo.native_pc_total,
            referrer_rebates_accrued: oo.referrer_rebates_accrued,
        }
    }
}

pub trait OpenOrdersAmounts {
    fn native_base_reserved(&self) -> u64;
    fn native_quote_reserved(&self) -> u64;
    fn native_base_free(&self) -> u64;
    fn native_quote_free(&self) -> u64;
    fn native_quote_free_plus_rebates(&self) -> u64;
    fn native_base_total(&self) -> u64;
    fn native_quote_total(&self) -> u64;
    fn native_quote_total_plus_rebates(&self) -> u64;
    fn native_rebates(&self) -> u64;
}

impl OpenOrdersAmounts for OpenOrdersSlim {
    fn native_base_reserved(&self) -> u64 {
        cm!(self.native_coin_total - self.native_coin_free)
    }
    fn native_quote_reserved(&self) -> u64 {
        cm!(self.native_pc_total - self.native_pc_free)
    }
    fn native_base_free(&self) -> u64 {
        self.native_coin_free
    }
    fn native_quote_free(&self) -> u64 {
        self.native_pc_free
    }
    fn native_quote_free_plus_rebates(&self) -> u64 {
        cm!(self.native_pc_free + self.referrer_rebates_accrued)
    }
    fn native_base_total(&self) -> u64 {
        self.native_coin_total
    }
    fn native_quote_total(&self) -> u64 {
        self.native_pc_total
    }
    fn native_quote_total_plus_rebates(&self) -> u64 {
        cm!(self.native_pc_total + self.referrer_rebates_accrued)
    }
    fn native_rebates(&self) -> u64 {
        self.referrer_rebates_accrued
    }
}

impl OpenOrdersAmounts for OpenOrders {
    fn native_base_reserved(&self) -> u64 {
        cm!(self.native_coin_total - self.native_coin_free)
    }
    fn native_quote_reserved(&self) -> u64 {
        cm!(self.native_pc_total - self.native_pc_free)
    }
    fn native_base_free(&self) -> u64 {
        self.native_coin_free
    }
    fn native_quote_free(&self) -> u64 {
        self.native_pc_free
    }
    fn native_quote_free_plus_rebates(&self) -> u64 {
        cm!(self.native_pc_free + self.referrer_rebates_accrued)
    }
    fn native_base_total(&self) -> u64 {
        self.native_coin_total
    }
    fn native_quote_total(&self) -> u64 {
        self.native_pc_total
    }
    fn native_quote_total_plus_rebates(&self) -> u64 {
        cm!(self.native_pc_total + self.referrer_rebates_accrued)
    }
    fn native_rebates(&self) -> u64 {
        self.referrer_rebates_accrued
    }
}

/// Copy paste a bunch of enums so that we could AnchorSerialize & AnchorDeserialize them

#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum Serum3SelfTradeBehavior {
    DecrementTake = 0,
    CancelProvide = 1,
    AbortTransaction = 2,
}

#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]

pub enum Serum3OrderType {
    Limit = 0,
    ImmediateOrCancel = 1,
    PostOnly = 2,
}
#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]

pub enum Serum3Side {
    Bid = 0,
    Ask = 1,
}

#[derive(Accounts)]
pub struct Serum3PlaceOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::Serum3PlaceOrder) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account at #2
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: The pubkey is checked and then it's passed to the serum cpi
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_event_queue: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_request_queue: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Validated by the serum cpi call
    pub market_quote_vault: UncheckedAccount<'info>,
    /// needed for the automatic settle_funds call
    /// CHECK: Validated by the serum cpi call
    pub market_vault_signer: UncheckedAccount<'info>,

    /// The bank that pays for the order, if necessary
    // token_index and payer_bank.vault == payer_vault is validated inline at #3
    #[account(mut, has_one = group)]
    pub payer_bank: AccountLoader<'info, Bank>,
    /// The bank vault that pays for the order, if necessary
    #[account(mut)]
    pub payer_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: The oracle can be one of several different account types
    #[account(address = payer_bank.load()?.oracle)]
    pub payer_oracle: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[allow(clippy::too_many_arguments)]
pub fn serum3_place_order(
    ctx: Context<Serum3PlaceOrder>,
    side: Serum3Side,
    limit_price: u64,
    max_base_qty: u64,
    max_native_quote_qty_including_fees: u64,
    self_trade_behavior: Serum3SelfTradeBehavior,
    order_type: Serum3OrderType,
    client_order_id: u64,
    limit: u16,
) -> Result<()> {
    let serum_market = ctx.accounts.serum_market.load()?;
    require!(
        !serum_market.is_reduce_only(),
        MangoError::MarketInReduceOnlyMode
    );

    //
    // Validation
    //
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
    }

    //
    // Pre-health computation
    //
    let mut account = ctx.accounts.account.load_full_mut()?;
    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health_cache =
            new_health_cache(&account.borrow(), &retriever).context("pre-withdraw init health")?;
        let pre_init_health = account.check_health_pre(&health_cache)?;
        Some((health_cache, pre_init_health))
    } else {
        None
    };

    //
    // Before-order tracking
    //

    let before_vault = ctx.accounts.payer_vault.amount;

    let before_oo = {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        OpenOrdersSlim::from_oo(&open_orders)
    };

    // Provide a readable error message in case the vault doesn't have enough tokens
    {
        let base_lot_size = load_market_state(
            &ctx.accounts.serum_market_external,
            &ctx.accounts.serum_program.key(),
        )?
        .coin_lot_size;

        let needed_amount = match side {
            Serum3Side::Ask => {
                cm!(max_base_qty * base_lot_size).saturating_sub(before_oo.native_base_free())
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
        limit_price: limit_price.try_into().unwrap(),
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

    let oo_difference = {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        let after_oo = OpenOrdersSlim::from_oo(&open_orders);

        emit!(Serum3OpenOrdersBalanceLogV2 {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            market_index: serum_market.market_index,
            base_token_index: serum_market.base_token_index,
            quote_token_index: serum_market.quote_token_index,
            base_total: after_oo.native_coin_total,
            base_free: after_oo.native_coin_free,
            quote_total: after_oo.native_pc_total,
            quote_free: after_oo.native_pc_free,
            referrer_rebates_accrued: after_oo.referrer_rebates_accrued,
        });

        OODifference::new(&before_oo, &after_oo)
    };

    //
    // After-order tracking
    //
    ctx.accounts.payer_vault.reload()?;
    let after_vault = ctx.accounts.payer_vault.amount;

    // Placing an order cannot increase vault balance
    require_gte!(before_vault, after_vault);

    let mut payer_bank = ctx.accounts.payer_bank.load_mut()?;

    // Enforce min vault to deposits ratio
    let withdrawn_from_vault = I80F48::from(cm!(before_vault - after_vault));
    let position_native = account
        .token_position_mut(payer_bank.token_index)?
        .0
        .native(&payer_bank);
    if withdrawn_from_vault > position_native {
        payer_bank.enforce_min_vault_to_deposits_ratio((*ctx.accounts.payer_vault).as_ref())?;
    }

    // Charge the difference in vault balance to the user's account
    let vault_difference = {
        let oracle_price =
            payer_bank.oracle_price(&AccountInfoRef::borrow(&ctx.accounts.payer_oracle)?, None)?;
        apply_vault_difference(
            ctx.accounts.account.key(),
            &mut account.borrow_mut(),
            serum_market.market_index,
            &mut payer_bank,
            after_vault,
            before_vault,
            Some(oracle_price),
        )?
    };

    //
    // Health check
    //
    if let Some((mut health_cache, pre_init_health)) = pre_health_opt {
        vault_difference.adjust_health_cache(&mut health_cache, &payer_bank)?;
        oo_difference.adjust_health_cache(&mut health_cache, &serum_market)?;
        account.check_health_post(&health_cache, pre_init_health)?;
    }

    // TODO: enforce min_vault_to_deposits_ratio

    Ok(())
}

pub struct OODifference {
    reserved_base_change: I80F48,
    reserved_quote_change: I80F48,
    free_base_change: I80F48,
    free_quote_change: I80F48,
}

impl OODifference {
    pub fn new(before_oo: &OpenOrdersSlim, after_oo: &OpenOrdersSlim) -> Self {
        Self {
            reserved_base_change: cm!(I80F48::from(after_oo.native_base_reserved())
                - I80F48::from(before_oo.native_base_reserved())),
            reserved_quote_change: cm!(I80F48::from(after_oo.native_quote_reserved())
                - I80F48::from(before_oo.native_quote_reserved())),
            free_base_change: cm!(I80F48::from(after_oo.native_base_free())
                - I80F48::from(before_oo.native_base_free())),
            free_quote_change: cm!(I80F48::from(after_oo.native_quote_free_plus_rebates())
                - I80F48::from(before_oo.native_quote_free_plus_rebates())),
        }
    }

    pub fn adjust_health_cache(
        &self,
        health_cache: &mut HealthCache,
        market: &Serum3Market,
    ) -> Result<()> {
        health_cache.adjust_serum3_reserved(
            market.market_index,
            market.base_token_index,
            self.reserved_base_change,
            self.free_base_change,
            market.quote_token_index,
            self.reserved_quote_change,
            self.free_quote_change,
        )
    }
}

pub struct VaultDifference {
    token_index: TokenIndex,
    native_change: I80F48,
}

impl VaultDifference {
    pub fn adjust_health_cache(&self, health_cache: &mut HealthCache, bank: &Bank) -> Result<()> {
        assert_eq!(bank.token_index, self.token_index);
        health_cache.adjust_token_balance(bank, self.native_change)?;
        Ok(())
    }
}

/// Called in settle_funds, place_order, liq_force_cancel to adjust token positions after
/// changing the vault balances
/// Also logs changes to token balances
pub fn apply_vault_difference(
    account_pk: Pubkey,
    account: &mut MangoAccountRefMut,
    serum_market_index: Serum3MarketIndex,
    bank: &mut Bank,
    vault_after: u64,
    vault_before: u64,
    oracle_price: Option<I80F48>,
) -> Result<VaultDifference> {
    let needed_change = cm!(I80F48::from(vault_after) - I80F48::from(vault_before));

    let (position, _) = account.token_position_mut(bank.token_index)?;
    let native_before = position.native(bank);
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();
    if needed_change >= 0 {
        bank.deposit(position, needed_change, now_ts)?;
    } else {
        bank.withdraw_without_fee(
            position,
            -needed_change,
            now_ts,
            oracle_price.unwrap(), // required for withdraws
        )?;
    }
    let native_after = position.native(bank);
    let native_change = cm!(native_after - native_before);
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
    *borrows_without_fee = cm!(old_value + new_borrows);

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
