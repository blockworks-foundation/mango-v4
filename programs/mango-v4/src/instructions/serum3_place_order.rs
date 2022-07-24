use crate::error::*;

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
    pub fn from_oo(oo: &OpenOrders) -> Self {
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

    #[account(mut, has_one = group)]
    pub account: MangoAccountAnchorLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated inline by checking against the pubkey stored in the account
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
        require!(
            account.fixed.is_owner_or_delegate(ctx.accounts.owner.key()),
            MangoError::SomeError
        );
        require!(!account.fixed.is_bankrupt(), MangoError::IsBankrupt);

        // Validate open_orders
        require!(
            account
                .serum3_find(serum_market.market_index)
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

    // Provide a readable error message in case the vault doesn't have enough tokens
    let (vault_amount, needed_amount) = match side {
        Serum3Side::Ask => (before_base_vault, max_base_qty),
        Serum3Side::Bid => (before_quote_vault, max_native_quote_qty_including_fees),
    };
    if vault_amount < needed_amount {
        return err!(MangoError::InsufficentBankVaultFunds).with_context(|| {
            format!(
                "bank vault does not have enough tokens, need {} but have {}",
                needed_amount, vault_amount
            )
        });
    }

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
        OpenOrdersSlim::from_oo(&open_orders)
    };
    cpi_place_order(ctx.accounts, order)?;

    {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        let after_oo = OpenOrdersSlim::from_oo(&open_orders);
        let mut account = ctx.accounts.account.load_mut()?;
        inc_maybe_loan(
            serum_market.market_index,
            &mut account.borrow_mut(),
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
    let mut account = ctx.accounts.account.load_mut()?;
    let vault_difference_result = {
        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        apply_vault_difference(
            &mut account.borrow_mut(),
            &mut base_bank,
            after_base_vault,
            before_base_vault,
            &mut quote_bank,
            after_quote_vault,
            before_quote_vault,
        )?
    };

    //
    // Health check
    //
    let retriever = new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
    let health = compute_health(&account.borrow(), HealthType::Init, &retriever)?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::HealthMustBePositive);

    vault_difference_result.deactivate_inactive_token_accounts(&mut account.borrow_mut());

    Ok(())
}

// if reserved has increased, then increase cached value by the increase in reserved
pub fn inc_maybe_loan(
    market_index: Serum3MarketIndex,
    account: &mut MangoAccountAccMut,
    before_oo: &OpenOrdersSlim,
    after_oo: &OpenOrdersSlim,
) {
    let serum3_account = account.serum3_find_mut(market_index).unwrap();

    if after_oo.native_coin_reserved() > before_oo.native_coin_reserved() {
        let native_coin_reserved_increase =
            after_oo.native_coin_reserved() - before_oo.native_coin_reserved();
        serum3_account.previous_native_coin_reserved =
            cm!(serum3_account.previous_native_coin_reserved + native_coin_reserved_increase);
    }

    if after_oo.native_pc_reserved() > before_oo.native_pc_reserved() {
        let reserved_pc_increase = after_oo.native_pc_reserved() - before_oo.native_pc_reserved();
        serum3_account.previous_native_pc_reserved =
            cm!(serum3_account.previous_native_pc_reserved + reserved_pc_increase);
    }
}

pub struct VaultDifferenceResult {
    base_raw_index: usize,
    base_active: bool,
    quote_raw_index: usize,
    quote_active: bool,
}

impl VaultDifferenceResult {
    pub fn deactivate_inactive_token_accounts(&self, account: &mut MangoAccountAccMut) {
        if !self.base_active {
            account.token_deactivate(self.base_raw_index);
        }
        if !self.quote_active {
            account.token_deactivate(self.quote_raw_index);
        }
    }
}

pub fn apply_vault_difference(
    account: &mut MangoAccountAccMut,
    base_bank: &mut Bank,
    after_base_vault: u64,
    before_base_vault: u64,
    quote_bank: &mut Bank,
    after_quote_vault: u64,
    before_quote_vault: u64,
) -> Result<VaultDifferenceResult> {
    // TODO: Applying the loan origination fee here may be too early: it should only be
    // charged if an order executes and the loan materializes? Otherwise MMs that place
    // an order without having the funds will be charged for each place_order!

    let (base_position, base_raw_index) = account.token_get_mut(base_bank.token_index)?;
    let base_change = I80F48::from(after_base_vault) - I80F48::from(before_base_vault);
    let base_active = base_bank.change_with_fee(base_position, base_change)?;

    let (quote_position, quote_raw_index) = account.token_get_mut(quote_bank.token_index)?;
    let quote_change = I80F48::from(after_quote_vault) - I80F48::from(before_quote_vault);
    let quote_active = quote_bank.change_with_fee(quote_position, quote_change)?;

    Ok(VaultDifferenceResult {
        base_raw_index,
        base_active,
        quote_raw_index,
        quote_active,
    })
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
