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

use crate::logs::{LoanOriginationFeeInstruction, WithdrawLoanOriginationFeeLog};

/// For loan origination fees bookkeeping purposes
pub struct OpenOrdersSlim {
    pub native_coin_free: u64,
    pub native_coin_total: u64,
    pub native_pc_free: u64,
    pub native_pc_total: u64,
    pub referrer_rebates_accrued: u64,
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
    fn native_quote_free(&self) -> u64; // includes settleable referrer rebates
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
        cm!(self.native_pc_free + self.referrer_rebates_accrued)
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
        cm!(self.native_pc_free + self.referrer_rebates_accrued)
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
    pub account: AccountLoaderDynamic<'info, MangoAccount>,
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

        // Validate open_orders
        require!(
            account
                .serum3_orders(serum_market.market_index)
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

    //
    // Pre-health computation
    //
    let mut account = ctx.accounts.account.load_mut()?;
    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let health_cache =
            new_health_cache(&account.borrow(), &retriever).context("pre-withdraw init health")?;
        let pre_health = health_cache.health(HealthType::Init);
        msg!("pre_health: {}", pre_health);
        account
            .fixed
            .maybe_recover_from_being_liquidated(pre_health);
        require!(
            !account.fixed.being_liquidated(),
            MangoError::BeingLiquidated
        );
        Some((health_cache, pre_health))
    } else {
        None
    };

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
    cpi_settle_funds(ctx.accounts)?;

    let oo_difference = {
        let oo_ai = &ctx.accounts.open_orders.as_ref();
        let open_orders = load_open_orders_ref(oo_ai)?;
        let after_oo = OpenOrdersSlim::from_oo(&open_orders);
        inc_maybe_loan(
            serum_market.market_index,
            &mut account.borrow_mut(),
            &before_oo,
            &after_oo,
        );
        OODifference::new(&before_oo, &after_oo)
    };

    //
    // After-order tracking
    //
    ctx.accounts.base_vault.reload()?;
    ctx.accounts.quote_vault.reload()?;
    let after_base_vault = ctx.accounts.base_vault.amount;
    let after_quote_vault = ctx.accounts.quote_vault.amount;

    // Charge the difference in vault balances to the user's account
    let vault_difference = {
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
    if let Some((mut health_cache, pre_health)) = pre_health_opt {
        vault_difference.adjust_health_cache(&mut health_cache)?;
        oo_difference.adjust_health_cache(&mut health_cache, &serum_market)?;

        let post_health = health_cache.health(HealthType::Init);
        msg!("post_health: {}", post_health);
        require!(
            post_health >= 0 || post_health > pre_health,
            MangoError::HealthMustBePositiveOrIncrease
        );
        account
            .fixed
            .maybe_recover_from_being_liquidated(post_health);
    }

    vault_difference.log_loan_origination_fees(
        &ctx.accounts.group.key(),
        &ctx.accounts.account.key(),
        LoanOriginationFeeInstruction::Serum3PlaceOrder,
    );
    vault_difference.deactivate_inactive_token_accounts(&mut account.borrow_mut());

    Ok(())
}

// if reserved has increased, then increase cached value by the increase in reserved
pub fn inc_maybe_loan(
    market_index: Serum3MarketIndex,
    account: &mut MangoAccountRefMut,
    before_oo: &OpenOrdersSlim,
    after_oo: &OpenOrdersSlim,
) {
    let serum3_account = account.serum3_orders_mut(market_index).unwrap();

    if after_oo.native_base_reserved() > before_oo.native_base_reserved() {
        let native_coin_reserved_increase =
            after_oo.native_base_reserved() - before_oo.native_base_reserved();
        serum3_account.previous_native_coin_reserved =
            cm!(serum3_account.previous_native_coin_reserved + native_coin_reserved_increase);
    }

    if after_oo.native_quote_reserved() > before_oo.native_quote_reserved() {
        let reserved_pc_increase =
            after_oo.native_quote_reserved() - before_oo.native_quote_reserved();
        serum3_account.previous_native_pc_reserved =
            cm!(serum3_account.previous_native_pc_reserved + reserved_pc_increase);
    }
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
            free_quote_change: cm!(I80F48::from(after_oo.native_quote_free())
                - I80F48::from(before_oo.native_quote_free())),
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

pub struct VaultDifferenceResult {
    base_raw_index: usize,
    base_index: TokenIndex,
    base_active: bool,
    quote_raw_index: usize,
    quote_index: TokenIndex,
    quote_active: bool,
    base_loan_origination_fee: I80F48,
    quote_loan_origination_fee: I80F48,
    base_native_change: I80F48,
    quote_native_change: I80F48,
}

impl VaultDifferenceResult {
    pub fn deactivate_inactive_token_accounts(&self, account: &mut MangoAccountRefMut) {
        if !self.base_active {
            account.deactivate_token_position(self.base_raw_index);
        }
        if !self.quote_active {
            account.deactivate_token_position(self.quote_raw_index);
        }
    }

    pub fn log_loan_origination_fees(
        &self,
        group: &Pubkey,
        account: &Pubkey,
        instruction: LoanOriginationFeeInstruction,
    ) {
        if self.base_loan_origination_fee.is_positive() {
            emit!(WithdrawLoanOriginationFeeLog {
                mango_group: *group,
                mango_account: *account,
                token_index: self.base_index,
                loan_origination_fee: self.base_loan_origination_fee.to_bits(),
                instruction,
            });
        }
        if self.quote_loan_origination_fee.is_positive() {
            emit!(WithdrawLoanOriginationFeeLog {
                mango_group: *group,
                mango_account: *account,
                token_index: self.quote_index,
                loan_origination_fee: self.quote_loan_origination_fee.to_bits(),
                instruction,
            });
        }
    }

    pub fn adjust_health_cache(&self, health_cache: &mut HealthCache) -> Result<()> {
        health_cache.adjust_token_balance(self.base_index, self.base_native_change)?;
        health_cache.adjust_token_balance(self.quote_index, self.quote_native_change)?;
        Ok(())
    }
}

pub fn apply_vault_difference(
    account: &mut MangoAccountRefMut,
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

    let (base_position, base_raw_index) = account.token_position_mut(base_bank.token_index)?;
    let base_native_before = base_position.native(&base_bank);
    let base_needed_change = cm!(I80F48::from(after_base_vault) - I80F48::from(before_base_vault));
    let (base_active, base_loan_origination_fee) =
        base_bank.change_with_fee(base_position, base_needed_change)?;
    let base_native_after = base_position.native(&base_bank);

    let (quote_position, quote_raw_index) = account.token_position_mut(quote_bank.token_index)?;
    let quote_native_before = quote_position.native(&quote_bank);
    let quote_needed_change =
        cm!(I80F48::from(after_quote_vault) - I80F48::from(before_quote_vault));
    let (quote_active, quote_loan_origination_fee) =
        quote_bank.change_with_fee(quote_position, quote_needed_change)?;
    let quote_native_after = quote_position.native(&quote_bank);

    Ok(VaultDifferenceResult {
        base_raw_index,
        base_index: base_bank.token_index,
        base_active,
        quote_raw_index,
        quote_index: quote_bank.token_index,
        quote_active,
        base_loan_origination_fee,
        quote_loan_origination_fee,
        base_native_change: cm!(base_native_after - base_native_before),
        quote_native_change: cm!(quote_native_after - quote_native_before),
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
