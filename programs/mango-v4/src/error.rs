use anchor_lang::prelude::*;
use core::fmt::Display;

// todo: group error blocks by kind
// todo: add comments which indicate decimal code for an error
#[error_code]
pub enum MangoError {
    #[msg("")]
    SomeError,
    #[msg("")]
    NotImplementedError,
    #[msg("checked math error")]
    MathError,
    #[msg("")]
    UnexpectedOracle,
    #[msg("oracle type cannot be determined")]
    UnknownOracleType,
    #[msg("")]
    InvalidFlashLoanTargetCpiProgram,
    #[msg("health must be positive")]
    HealthMustBePositive,
    #[msg("health must be positive or not decrease")]
    HealthMustBePositiveOrIncrease, // outdated name is kept for backwards compatibility
    #[msg("health must be negative")]
    HealthMustBeNegative,
    #[msg("the account is bankrupt")]
    IsBankrupt,
    #[msg("the account is not bankrupt")]
    IsNotBankrupt,
    #[msg("no free token position index")]
    NoFreeTokenPositionIndex,
    #[msg("no free serum3 open orders index")]
    NoFreeSerum3OpenOrdersIndex,
    #[msg("no free perp position index")]
    NoFreePerpPositionIndex,
    #[msg("serum3 open orders exist already")]
    Serum3OpenOrdersExistAlready,
    #[msg("bank vault has insufficent funds")]
    InsufficentBankVaultFunds,
    #[msg("account is currently being liquidated")]
    BeingLiquidated,
    #[msg("invalid bank")]
    InvalidBank,
    #[msg("account profitability is mismatched")]
    ProfitabilityMismatch,
    #[msg("cannot settle with self")]
    CannotSettleWithSelf,
    #[msg("perp position does not exist")]
    PerpPositionDoesNotExist,
    #[msg("max settle amount must be greater than zero")]
    MaxSettleAmountMustBeGreaterThanZero,
    #[msg("the perp position has open orders or unprocessed fill events")]
    HasOpenPerpOrders,
    #[msg("an oracle does not reach the confidence threshold")]
    OracleConfidence,
    #[msg("an oracle is stale")]
    OracleStale,
    #[msg("settlement amount must always be positive")]
    SettlementAmountMustBePositive,
    #[msg("bank utilization has reached limit")]
    BankBorrowLimitReached,
    #[msg("bank net borrows has reached limit - this is an intermittent error - the limit will reset regularly")]
    BankNetBorrowsLimitReached,
    #[msg("token position does not exist")]
    TokenPositionDoesNotExist,
    #[msg("token deposits into accounts that are being liquidated must bring their health above the init threshold")]
    DepositsIntoLiquidatingMustRecover,
    #[msg("token is in reduce only mode")]
    TokenInReduceOnlyMode,
    #[msg("market is in reduce only mode")]
    MarketInReduceOnlyMode,
    #[msg("group is halted")]
    GroupIsHalted,
    #[msg("the perp position has non-zero base lots")]
    PerpHasBaseLots,
    #[msg("there are open or unsettled serum3 orders")]
    HasOpenOrUnsettledSerum3Orders,
    #[msg("has liquidatable token position")]
    HasLiquidatableTokenPosition,
    #[msg("has liquidatable perp base position")]
    HasLiquidatablePerpBasePosition,
    #[msg("has liquidatable positive perp pnl")]
    HasLiquidatablePositivePerpPnl,
    #[msg("account is frozen")]
    AccountIsFrozen,
    #[msg("Init Asset Weight can't be negative")]
    InitAssetWeightCantBeNegative,
    #[msg("has open perp taker fills")]
    HasOpenPerpTakerFills,
    #[msg("deposit crosses the current group deposit limit")]
    DepositLimit,
    #[msg("instruction is disabled")]
    IxIsDisabled,
    #[msg("no liquidatable perp base position")]
    NoLiquidatablePerpBasePosition,
    #[msg("perp order id not found on the orderbook")]
    PerpOrderIdNotFound,
    #[msg("HealthRegions allow only specific instructions between Begin and End")]
    HealthRegionBadInnerInstruction,
    #[msg("token is in force close")]
    TokenInForceClose,
    #[msg("incorrect number of health accounts")]
    InvalidHealthAccountCount,
    #[msg("would self trade")]
    WouldSelfTrade,
    #[msg("token conditional swap oracle price is not in execution range")]
    TokenConditionalSwapPriceNotInRange,
    #[msg("token conditional swap is expired")]
    TokenConditionalSwapExpired,
    #[msg("token conditional swap is not available yet")]
    TokenConditionalSwapNotStarted,
    #[msg("token conditional swap was already started")]
    TokenConditionalSwapAlreadyStarted,
    #[msg("token conditional swap it not set")]
    TokenConditionalSwapNotSet,
    #[msg("token conditional swap trigger did not reach min_buy_token")]
    TokenConditionalSwapMinBuyTokenNotReached,
    #[msg("token conditional swap cannot pay incentive")]
    TokenConditionalSwapCantPayIncentive,
    #[msg("token conditional swap taker price is too low")]
    TokenConditionalSwapTakerPriceTooLow,
    #[msg("token conditional swap index and id don't match")]
    TokenConditionalSwapIndexIdMismatch,
    #[msg("token conditional swap volume is too small compared to the cost of starting it")]
    TokenConditionalSwapTooSmallForStartIncentive,
    #[msg("token conditional swap type cannot be started")]
    TokenConditionalSwapTypeNotStartable,
    #[msg("a bank in the health account list should be writable but is not")]
    HealthAccountBankNotWritable,
    #[msg("the market does not allow limit orders too far from the current oracle value")]
    Serum3PriceBandExceeded,
    #[msg("deposit crosses the token's deposit limit")]
    BankDepositLimit,
    #[msg("delegates can only withdraw to the owner's associated token account")]
    DelegateWithdrawOnlyToOwnerAta,
    #[msg("delegates can only withdraw if they close the token position")]
    DelegateWithdrawMustClosePosition,
    #[msg("delegates can only withdraw small amounts")]
    DelegateWithdrawSmall,
    #[msg("The provided CLMM oracle is not valid")]
    InvalidCLMMOracle,
    #[msg("invalid usdc/usd feed provided for the CLMM oracle")]
    InvalidFeedForCLMMOracle,
    #[msg("Pyth USDC/USD or SOL/USD feed not found (required by CLMM oracle)")]
    MissingFeedForCLMMOracle,
    #[msg("the asset does not allow liquidation")]
    TokenAssetLiquidationDisabled,
}

impl MangoError {
    pub fn error_code(&self) -> u32 {
        (*self).into()
    }
}

pub trait IsAnchorErrorWithCode {
    fn is_anchor_error_with_code(&self, code: u32) -> bool;
    fn is_oracle_error(&self) -> bool;
}

impl<T> IsAnchorErrorWithCode for anchor_lang::Result<T> {
    fn is_anchor_error_with_code(&self, code: u32) -> bool {
        match self {
            Err(Error::AnchorError(error)) => error.error_code_number == code,
            _ => false,
        }
    }
    fn is_oracle_error(&self) -> bool {
        match self {
            Err(Error::AnchorError(e)) => {
                e.error_code_number == MangoError::OracleConfidence.error_code()
                    || e.error_code_number == MangoError::OracleStale.error_code()
            }
            _ => false,
        }
    }
}

pub trait Contextable {
    /// Add a context string `c` to a Result or Error
    ///
    /// Example: foo().context("calling foo")?;
    fn context(self, c: impl Display) -> Self;

    /// Like `context()`, but evaluate the context string lazily
    ///
    /// Use this if it's expensive to generate, like a format!() call.
    fn with_context<C, F>(self, c: F) -> Self
    where
        C: Display,
        F: FnOnce() -> C;
}

impl Contextable for Error {
    fn context(self, c: impl Display) -> Self {
        match self {
            Error::AnchorError(err) => Error::AnchorError(AnchorError {
                error_msg: if err.error_msg.is_empty() {
                    format!("{}", c)
                } else {
                    format!("{}; {}", err.error_msg, c)
                },
                ..err
            }),
            // Maybe wrap somehow?
            Error::ProgramError(err) => Error::ProgramError(err),
        }
    }
    fn with_context<C, F>(self, c: F) -> Self
    where
        C: Display,
        F: FnOnce() -> C,
    {
        self.context(c())
    }
}

impl<T> Contextable for Result<T> {
    fn context(self, c: impl Display) -> Self {
        if let Err(err) = self {
            Err(err.context(c))
        } else {
            self
        }
    }
    fn with_context<C, F>(self, c: F) -> Self
    where
        C: Display,
        F: FnOnce() -> C,
    {
        if let Err(err) = self {
            Err(err.context(c()))
        } else {
            self
        }
    }
}

/// Creates an Error with a particular message, using format!() style arguments
///
/// Example: error_msg!("index {} not found", index)
#[macro_export]
macro_rules! error_msg {
    ($($arg:tt)*) => {
        error!(MangoError::SomeError).context(format!($($arg)*))
    };
}

/// Creates an Error with a particular message, using format!() style arguments
///
/// Example: error_msg_typed!(TokenPositionMissing, "index {} not found", index)
#[macro_export]
macro_rules! error_msg_typed {
    ($code:expr, $($arg:tt)*) => {
        error!($code).context(format!($($arg)*))
    };
}

/// Like anchor's require!(), but with a customizable message
///
/// Example: require_msg!(condition, "the condition on account {} was violated", account_key);
#[macro_export]
macro_rules! require_msg {
    ($invariant:expr, $($arg:tt)*) => {
        if !($invariant) {
            return Err(error_msg!($($arg)*));
        }
    };
}

/// Like anchor's require!(), but with a customizable message and type
///
/// Example: require_msg_typed!(condition, "the condition on account {} was violated", account_key);
#[macro_export]
macro_rules! require_msg_typed {
    ($invariant:expr, $code:expr, $($arg:tt)*) => {
        if !($invariant) {
            return Err(error_msg_typed!($code, $($arg)*));
        }
    };
}

pub use error_msg;
pub use error_msg_typed;
pub use require_msg;
pub use require_msg_typed;
