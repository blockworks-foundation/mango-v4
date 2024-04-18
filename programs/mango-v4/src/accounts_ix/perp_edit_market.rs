use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpEditMarket<'info> {
    pub group: AccountLoader<'info, Group>,
    // group <-> admin relation is checked at #1
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    /// The oracle account is optional and only used when reset_stable_price is set.
    ///
    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    /// The fallback oracle account is optional and only used when set_fallback_oracle is true.
    ///
    /// CHECK: The fallback oracle can be one of several different account types
    pub fallback_oracle: UncheckedAccount<'info>,
}
