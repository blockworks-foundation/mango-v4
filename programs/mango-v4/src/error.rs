use anchor_lang::prelude::*;

// todo: group error blocks by kind
// todo: add comments which indicate decimal code for an error
#[error_code]
pub enum MangoError {
    #[msg("")]
    SomeError,
    #[msg("")]
    UnknownOracle,
}
