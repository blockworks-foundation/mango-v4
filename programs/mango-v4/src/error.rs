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
    #[msg("health must be positive or increase")]
    HealthMustBePositiveOrIncrease,
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

/// Like anchor's require!(), but with a customizable message
///
/// Example: require!(condition, "the condition on account {} was violated", account_key);
#[macro_export]
macro_rules! require_msg {
    ($invariant:expr, $($arg:tt)*) => {
        if !($invariant) {
            return Err(error_msg!($($arg)*));
        }
    };
}

pub use error_msg;
pub use require_msg;
