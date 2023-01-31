use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::mem::size_of;

// TODO: Assuming we allow up to 65536 different tokens
pub type TokenIndex = u16;
pub const QUOTE_TOKEN_INDEX: TokenIndex = 0;

#[account(zero_copy(safe_bytemuck_derives))]
#[derive(Debug)]
pub struct Group {
    // ABI: Clients rely on this being at offset 8
    pub creator: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub group_num: u32,

    pub admin: Pubkey,

    // TODO: unused, use case - listing shit tokens with conservative parameters (mostly defaults)
    pub fast_listing_admin: Pubkey,

    pub padding: [u8; 4],

    pub insurance_vault: Pubkey,
    pub insurance_mint: Pubkey,

    pub bump: u8,

    pub testing: u8,

    pub version: u8,

    pub padding2: [u8; 5],

    pub address_lookup_tables: [Pubkey; 20],

    pub security_admin: Pubkey,

    // Deposit limit for a mango account in quote native, enforced on quote value of account assets
    // Set to 0 to disable, which also means by default there is no limit
    pub deposit_limit_quote: u64,

    // Map of ixs and their state of gating
    // 0 is chosen as enabled, becase we want to start out with all ixs enabled, 1 is disabled
    pub ix_gate: u128,

    pub reserved: [u8; 1864],
}
const_assert_eq!(
    size_of::<Group>(),
    32 + 4 + 32 * 2 + 4 + 32 * 2 + 3 + 5 + 20 * 32 + 32 + 8 + 16 + 1864
);
const_assert_eq!(size_of::<Group>(), 2736);
const_assert_eq!(size_of::<Group>() % 8, 0);

impl Group {
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
    Serum3EditMarket = 31,
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
