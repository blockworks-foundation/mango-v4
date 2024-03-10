use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

// TODO: Assuming we allow up to 65536 different tokens
pub type TokenIndex = u16;

/// This token index is supposed to be the token that oracles quote in.
///
/// In practice this is set to the USDC token index, and that is wrong: actually
/// oracles quote in USD. Any use of this constant points to a potentially
/// incorrect assumption.
pub const QUOTE_TOKEN_INDEX: TokenIndex = 0;

/// The token index used for the insurance fund.
///
/// We should eventually generalize insurance funds.
pub const INSURANCE_TOKEN_INDEX: TokenIndex = 0;

/// The token index used for settling perp markets.
///
/// We should eventually generalize to make the whole perp quote (and settle) token
/// configurable.
pub const PERP_SETTLE_TOKEN_INDEX: TokenIndex = 0;

/// The token index used in AccountBuybackFeesWithMngo to exchange for MNGO
pub const FEE_BUYBACK_QUOTE_TOKEN_INDEX: TokenIndex = 0;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Group {
    // ABI: Clients rely on this being at offset 8
    pub creator: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub group_num: u32,

    pub admin: Pubkey,

    // TODO: unused, use case - listing shit tokens with conservative parameters (mostly defaults)
    pub fast_listing_admin: Pubkey,

    // This is the token index of the mngo token listed on the group
    pub mngo_token_index: TokenIndex,
    pub padding: [u8; 2],

    pub insurance_vault: Pubkey,
    pub insurance_mint: Pubkey,

    pub bump: u8,

    pub testing: u8,

    pub version: u8,

    // Buyback fees with Mngo: allow exchanging fees with mngo at a bonus
    pub buyback_fees: u8,
    // Buyback fees with Mngo: how much should the bonus be,
    // e.g. a bonus factor of 1.2 means 120$ worth fees could be swapped for mngo worth 100$ at current market price
    pub buyback_fees_mngo_bonus_factor: f32,

    pub address_lookup_tables: [Pubkey; 20],

    pub security_admin: Pubkey,

    // Deposit limit for a mango account in quote native, enforced on quote value of account assets
    // Set to 0 to disable, which also means by default there is no limit
    pub deposit_limit_quote: u64,

    // Map of ixs and their state of gating
    // 0 is chosen as enabled, becase we want to start out with all ixs enabled, 1 is disabled
    pub ix_gate: u128,

    // Buyback fees with Mngo:
    // A mango account which would be counter party for settling fees with mngo
    // This ensures that the system doesn't have a net deficit of tokens
    // The workflow should be something like this
    // - the dao deposits quote tokens in its respective mango account
    // - the user deposits some mngo tokens in his mango account
    // - the user then claims quote for mngo at a bonus rate
    pub buyback_fees_swap_mango_account: Pubkey,

    /// Number of seconds after which fees that could be used with the fees buyback feature expire.
    ///
    /// The actual expiry is staggered such that the fees users accumulate are always
    /// available for at least this interval - but may be available for up to twice this time.
    ///
    /// When set to 0, there's no expiry of buyback fees.
    pub buyback_fees_expiry_interval: u64,

    /// Fast-listings are limited per week, this is the start of the current fast-listing interval
    /// in seconds since epoch
    pub fast_listing_interval_start: u64,

    /// Number of fast listings that happened this interval
    pub fast_listings_in_interval: u16,

    /// Number of fast listings that are allowed per interval
    pub allowed_fast_listings_per_interval: u16,

    pub padding2: [u8; 4],

    /// Intervals in which collateral fee is applied
    pub collateral_fee_interval: u64,

    pub reserved: [u8; 1800],
}
const_assert_eq!(
    size_of::<Group>(),
    32 + 4
        + 32 * 2
        + 4
        + 32 * 2
        + 4
        + 4
        + 20 * 32
        + 32
        + 8
        + 16
        + 32
        + 8
        + 8
        + 2 * 2
        + 4
        + 8
        + 1800
);
const_assert_eq!(size_of::<Group>(), 2736);
const_assert_eq!(size_of::<Group>() % 8, 0);

impl Group {
    pub fn buyback_fees(&self) -> bool {
        self.buyback_fees == 1
    }

    pub fn is_testing(&self) -> bool {
        self.testing == 1
    }

    pub fn multiple_banks_supported(&self) -> bool {
        self.is_testing() || self.version > 1
    }

    pub fn serum3_supported(&self) -> bool {
        self.is_testing() || self.version > 0
    }

    pub fn perps_supported(&self) -> bool {
        self.is_testing() || self.version > 1
    }

    pub fn is_ix_enabled(&self, ix: IxGate) -> bool {
        self.ix_gate & (1 << ix as u128) == 0
    }

    pub fn openbook_v2_supported(&self) -> bool {
        self.is_testing()
    }
}

/// Enum for lookup into ix gate
/// note:
/// total ix files 56,
/// ix files included 48,
/// ix files not included 8,
/// - Benchmark,
/// - ComputeAccountData,
/// - GroupCreate
/// - GroupEdit
/// - IxGateSet,
/// - PerpZeroOut,
/// - PerpEditMarket,
/// - TokenEdit,
#[derive(Copy, Clone, Debug)]
pub enum IxGate {
    AccountClose = 0,
    AccountCreate = 1,
    AccountEdit = 2,
    AccountExpand = 3,
    AccountToggleFreeze = 4,
    AltExtend = 5,
    AltSet = 6,
    FlashLoan = 7,
    GroupClose = 8,
    GroupCreate = 9,
    HealthRegion = 10,
    PerpCancelAllOrders = 11,
    PerpCancelAllOrdersBySide = 12,
    PerpCancelOrder = 13,
    PerpCancelOrderByClientOrderId = 14,
    PerpCloseMarket = 15,
    PerpConsumeEvents = 16,
    PerpCreateMarket = 17,
    PerpDeactivatePosition = 18,
    PerpLiqBaseOrPositivePnl = 19,
    PerpLiqForceCancelOrders = 20,
    PerpLiqNegativePnlOrBankruptcy = 21,
    PerpPlaceOrder = 22,
    PerpSettleFees = 23,
    PerpSettlePnl = 24,
    PerpUpdateFunding = 25,
    Serum3CancelAllOrders = 26,
    Serum3CancelOrder = 27,
    Serum3CloseOpenOrders = 28,
    Serum3CreateOpenOrders = 29,
    Serum3DeregisterMarket = 30,
    Serum3EditMarket = 31, // Note: Unused, and should never be used, added mistakenly.
    Serum3LiqForceCancelOrders = 32,
    Serum3PlaceOrder = 33,
    Serum3RegisterMarket = 34,
    Serum3SettleFunds = 35,
    StubOracleClose = 36,
    StubOracleCreate = 37,
    StubOracleSet = 38,
    TokenAddBank = 39,
    TokenDeposit = 40,
    TokenDeregister = 41,
    TokenLiqBankruptcy = 42,
    TokenLiqWithToken = 43,
    TokenRegister = 44,
    TokenRegisterTrustless = 45,
    TokenUpdateIndexAndRate = 46,
    TokenWithdraw = 47,
    AccountBuybackFeesWithMngo = 48,
    TokenForceCloseBorrowsWithToken = 49,
    PerpForceClosePosition = 50,
    GroupWithdrawInsuranceFund = 51,
    TokenConditionalSwapCreate = 52,
    TokenConditionalSwapTrigger = 53,
    TokenConditionalSwapCancel = 54,
    OpenbookV2CancelOrder = 55,
    OpenbookV2CloseOpenOrders = 56,
    OpenbookV2CreateOpenOrders = 57,
    OpenbookV2DeregisterMarket = 58,
    OpenbookV2EditMarket = 59,
    OpenbookV2LiqForceCancelOrders = 60,
    OpenbookV2PlaceOrder = 61,
    OpenbookV2PlaceTakeOrder = 62,
    OpenbookV2RegisterMarket = 63,
    OpenbookV2SettleFunds = 64,
    AdminTokenWithdrawFees = 65,
    AdminPerpWithdrawFees = 66,
    AccountSizeMigration = 67,
    TokenConditionalSwapStart = 68,
    TokenConditionalSwapCreatePremiumAuction = 69,
    TokenConditionalSwapCreateLinearAuction = 70,
    Serum3PlaceOrderV2 = 71,
    TokenForceWithdraw = 72,
    // NOTE: Adding new variants requires matching changes in ts and the ix_gate_set instruction.
}

// note: using creator instead of admin, since admin can be changed
#[macro_export]
macro_rules! group_seeds {
    ( $group:expr ) => {
        &[
            b"Group".as_ref(),
            $group.creator.as_ref(),
            &$group.group_num.to_le_bytes(),
            &[$group.bump],
        ]
    };
}

pub use group_seeds;
