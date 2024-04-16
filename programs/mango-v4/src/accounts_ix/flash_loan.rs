use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_spl::{associated_token::AssociatedToken, token::Token};

pub mod jupiter_mainnet_6 {
    use solana_program::declare_id;
    declare_id!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
}

pub mod jupiter_mainnet_4 {
    use solana_program::declare_id;
    declare_id!("JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB");
}
pub mod jupiter_mainnet_3 {
    use solana_program::declare_id;
    declare_id!("JUP3c2Uh3WA4Ng34tw6kPd2G4C5BB21Xo36Je1s32Ph");
}

/// Sets up mango vaults for flash loan
///
/// In addition to these accounts, there must be remaining_accounts:
/// 1. N banks (writable)
/// 2. N vaults (writable), matching the banks
/// 3. N token accounts (writable), in the same order as the vaults,
///    the loaned funds are transfered into these
/// 4. the mango group
#[derive(Accounts)]
pub struct FlashLoanBegin<'info> {
    #[account(
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    // owner is checked at #1
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,

    /// Instructions Sysvar for instruction introspection
    /// CHECK: fixed instructions sysvar account
    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct FlashLoanSwapBegin<'info> {
    #[account(
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    // owner is checked at #1
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: bank/vault/token account in remaining accounts match against this
    pub input_mint: UncheckedAccount<'info>,
    /// CHECK: bank/vault/token account in remaining accounts match against this
    pub output_mint: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Instructions Sysvar for instruction introspection
    /// CHECK: fixed instructions sysvar account
    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// Finalizes a flash loan
///
/// In addition to these accounts, there must be remaining_accounts:
/// 1. health accounts, and every bank that also appeared in FlashLoanBegin must be writable
/// 2. N vaults (writable), matching what was in FlashLoanBegin
/// 3. N token accounts (writable), matching what was in FlashLoanBegin;
///    the `owner` must have authority to transfer tokens out of them
/// 4. the mango group
#[derive(Accounts)]
pub struct FlashLoanEnd<'info> {
    #[account(
        mut,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    // owner is checked at #1
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(PartialEq, Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum FlashLoanType {
    /// An arbitrary flash loan
    Unknown,
    /// A flash loan used for a swap where one token is exchanged for another.
    ///
    /// Deposits in this type get charged the flash_loan_swap_fee_rate
    Swap,
    /// Like Swap, but without the flash_loan_swap_fee_rate
    SwapWithoutFee,
}
