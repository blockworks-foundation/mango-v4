use crate::accounts_zerocopy::*;
use crate::error::MangoError;

use crate::serum3_cpi::load_open_orders_ref;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use checked_math as cm;
use fixed::types::I80F48;
use num_enum::IntoPrimitive;
use num_enum::TryFromPrimitive;
use serum_dex::instruction::NewOrderInstructionV3;
use serum_dex::matching::Side;
use serum_dex::state::OpenOrders;

/// For loan origination fees bookkeeping purposes
pub struct OpenOrdersSlim {
    pub native_coin_free: u64,
    pub native_coin_total: u64,
    pub native_pc_free: u64,
    pub native_pc_total: u64,
}
impl OpenOrdersSlim {
    pub fn fromOO(oo: &OpenOrders) -> Self {
        Self {
            native_coin_free: oo.native_coin_free,
            native_coin_total: oo.native_coin_total,
            native_pc_free: oo.native_pc_free,
            native_pc_total: oo.native_pc_total,
        }
    }
}

pub trait OpenOrdersReserved {
    fn native_coin_reserved(&self) -> u64;
    fn native_pc_reserved(&self) -> u64;
}

impl OpenOrdersReserved for OpenOrdersSlim {
    fn native_coin_reserved(&self) -> u64 {
        self.native_coin_total - self.native_coin_free
    }
    fn native_pc_reserved(&self) -> u64 {
        self.native_pc_total - self.native_pc_free
    }
}

impl OpenOrdersReserved for OpenOrders {
    fn native_coin_reserved(&self) -> u64 {
        self.native_coin_total - self.native_coin_free
    }
    fn native_pc_reserved(&self) -> u64 {
        self.native_pc_total - self.native_pc_free
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
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    // Validated inline
    #[account(mut)]
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_event_queue: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_request_queue: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_quote_vault: UncheckedAccount<'info>,
    // needed for the automatic settle_funds call
    pub market_vault_signer: UncheckedAccount<'info>,

    // TODO: do we need to pass both, or just payer?
    // TODO: if we potentially settle immediately, they all need to be mut?
    // TODO: Can we reduce the number of accounts by requiring the banks
    //       to be in the remainingAccounts (where they need to be anyway, for
    //       health checks - but they need to be mut)
    // token_index and bank.vault == vault is validated inline
    #[account(mut, has_one = group)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, has_one = group)]
    pub base_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub base_vault: Box<Account<'info, TokenAccount>>,

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

    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
        require!(account.is_bankrupt == 0, MangoError::IsBankrupt);

        // Validate open_orders
        require!(
            account
                .serum3
                .find(serum_market.market_index)
                .ok_or_else(|| error!(MangoError::SomeError))?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );

        // Validate banks and vaults
        let quote_bank = ctx.accounts.quote_bank.load()?;
        require!(
            quote_bank.vault == ctx.accounts.quote_vault.key(),
            MangoError::SomeError
        );
        require!(
            quote_bank.token_index == serum_market.quote_token_index,
            MangoError::SomeError
        );
        let base_bank = ctx.accounts.base_bank.load()?;
        require!(
            base_bank.vault == ctx.accounts.base_vault.key(),
            MangoError::SomeError
        );
        require!(
            base_bank.token_index == serum_market.base_token_index,
            MangoError::SomeError
        );
    }

    //
    // Before-order tracking
    //

    let before_base_vault = ctx.accounts.base_vault.amount;
    let before_quote_vault = ctx.accounts.quote_vault.amount;

    // TODO: pre-health check

    //
    // Apply the order to serum. Also immediately settle, in case the order
    // matched against an existing other order.
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
    };

    let before_oo = {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        let before_oo = OpenOrdersSlim::fromOO(&open_orders);
        cpi_place_order(ctx.accounts, order)?;
        before_oo
    };

    {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        let after_oo = OpenOrdersSlim::fromOO(&open_orders);
        let mut account = ctx.accounts.account.load_mut()?;
        inc_maybe_loan(
            serum_market.market_index,
            &mut account,
            &before_oo,
            &after_oo,
        );
    }

    cpi_settle_funds(ctx.accounts)?;

    //
    // After-order tracking
    //
    ctx.accounts.base_vault.reload()?;
    ctx.accounts.quote_vault.reload()?;
    let after_base_vault = ctx.accounts.base_vault.amount;
    let after_quote_vault = ctx.accounts.quote_vault.amount;

    // Charge the difference in vault balances to the user's account
    apply_vault_difference(
        ctx.accounts.account.load_mut()?,
        ctx.accounts.base_bank.load_mut()?,
        after_base_vault,
        before_base_vault,
        ctx.accounts.quote_bank.load_mut()?,
        after_quote_vault,
        before_quote_vault,
    )?;

    //
    // Health check
    //
    let account = ctx.accounts.account.load()?;
    let health =
        compute_health_from_fixed_accounts(&account, HealthType::Init, ctx.remaining_accounts)?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::HealthMustBePositive);

    Ok(())
}

// if reserved has increased, then increase cached value by the increase in reserved
pub fn inc_maybe_loan(
    market_index: Serum3MarketIndex,
    account: &mut MangoAccount,
    before_oo: &OpenOrdersSlim,
    after_oo: &OpenOrdersSlim,
) {
    let serum3_account = account.serum3.find_mut(market_index).unwrap();

    if after_oo.native_coin_reserved() > before_oo.native_coin_reserved() {
        let native_coin_reserved_increase =
            after_oo.native_coin_reserved() - before_oo.native_coin_reserved();
        serum3_account.native_coin_reserved_cached =
            cm!(serum3_account.native_coin_reserved_cached + native_coin_reserved_increase);
    }

    if after_oo.native_pc_reserved() > before_oo.native_pc_reserved() {
        let reserved_pc_increase = after_oo.native_pc_reserved() - before_oo.native_pc_reserved();
        serum3_account.native_pc_reserved_cached =
            cm!(serum3_account.native_pc_reserved_cached + reserved_pc_increase);
    }
}

pub fn apply_vault_difference(
    mut account: std::cell::RefMut<MangoAccount>,
    mut base_bank: std::cell::RefMut<Bank>,
    after_base_vault: u64,
    before_base_vault: u64,
    mut quote_bank: std::cell::RefMut<Bank>,
    after_quote_vault: u64,
    before_quote_vault: u64,
) -> Result<()> {
    // TODO: Applying the loan origination fee here may be too early: it should only be
    // charged if an order executes and the loan materializes? Otherwise MMs that place
    // an order without having the funds will be charged for each place_order!

    let base_position = account.tokens.get_mut(base_bank.token_index)?;
    let base_change = I80F48::from(after_base_vault) - I80F48::from(before_base_vault);
    base_bank.change_with_fee(base_position, base_change)?;

    let quote_position = account.tokens.get_mut(quote_bank.token_index)?;
    let quote_change = I80F48::from(after_quote_vault) - I80F48::from(before_quote_vault);
    quote_bank.change_with_fee(quote_position, quote_change)?;

    Ok(())
}

fn cpi_place_order(ctx: &Serum3PlaceOrder, order: NewOrderInstructionV3) -> Result<()> {
    use crate::serum3_cpi;

    let order_payer_token_account = match order.side {
        Side::Bid => &ctx.quote_vault,
        Side::Ask => &ctx.base_vault,
    };

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
        order_payer_token_account: order_payer_token_account.to_account_info(),
        user_authority: ctx.group.to_account_info(),
    }
    .call(&group, order)
}

fn cpi_settle_funds(ctx: &Serum3PlaceOrder) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::SettleFunds {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
        base_vault: ctx.market_base_vault.to_account_info(),
        quote_vault: ctx.market_quote_vault.to_account_info(),
        user_base_wallet: ctx.base_vault.to_account_info(),
        user_quote_wallet: ctx.quote_vault.to_account_info(),
        vault_signer: ctx.market_vault_signer.to_account_info(),
        token_program: ctx.token_program.to_account_info(),
    }
    .call(&group)
}
