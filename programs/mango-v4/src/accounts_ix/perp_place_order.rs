use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct PerpPlaceOrder<'info> {
    #[account(
        constraint = group.load()?.is_ix_enabled(IxGate::PerpPlaceOrder) @ MangoError::IxIsDisabled,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        constraint = account.load()?.is_operational() @ MangoError::AccountIsFrozen
        // owner is checked at #1
    )]
    pub account: AccountLoader<'info, MangoAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = bids,
        has_one = asks,
        has_one = event_queue,
        has_one = oracle,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,
    #[account(mut)]
    pub bids: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub asks: AccountLoader<'info, BookSide>,
    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,

    /// CHECK: The oracle can be one of several different account types and the pubkey is checked above
    pub oracle: UncheckedAccount<'info>,
}

pub struct PerpPlaceOrderAccountInfos<'info, 'remaining, 'a> {
    pub group: &'a AccountInfo<'info>,
    pub account: &'a AccountInfo<'info>,

    pub perp_market: &'a AccountInfo<'info>,
    pub bids: &'a AccountInfo<'info>,
    pub asks: &'a AccountInfo<'info>,
    pub event_queue: &'a AccountInfo<'info>,

    pub oracle: &'a AccountInfo<'info>,

    pub remaining: &'remaining [AccountInfo<'info>],
}

pub struct PerpPlaceOrderAccounts<'info, 'remaining> {
    pub group: AccountLoader<'info, Group>,
    pub account: AccountLoader<'info, MangoAccountFixed>,

    pub perp_market: AccountLoader<'info, PerpMarket>,
    pub bids: AccountLoader<'info, BookSide>,
    pub asks: AccountLoader<'info, BookSide>,
    pub event_queue: AccountLoader<'info, EventQueue>,

    pub oracle: UncheckedAccount<'info>,

    pub remaining: &'remaining [AccountInfo<'info>],
}

impl<'info, 'remaining> PerpPlaceOrderAccounts<'info, 'remaining> {
    pub fn from_account_infos<'a>(
        ais: PerpPlaceOrderAccountInfos<'info, 'remaining, 'a>,
    ) -> Result<Self> {
        let program_id = &crate::id();
        Ok(Self {
            group: AccountLoader::try_from(ais.group)?,
            account: AccountLoader::try_from(ais.account)?,
            perp_market: AccountLoader::try_from(ais.perp_market)?,
            bids: AccountLoader::try_from(ais.bids)?,
            asks: AccountLoader::try_from(ais.asks)?,
            event_queue: AccountLoader::try_from(ais.event_queue)?,
            oracle: UncheckedAccount::try_from(ais.oracle.clone()),
            remaining: ais.remaining,
        })
    }

    // This duplicates the anchor account constraints, but is necessary since indirect
    // calls from trigger order execution won't go through the normal accounts struct.
    // (they can't, because the normal struct requires a signing owner)
    pub fn validate(&self, owner: &Pubkey) -> Result<()> {
        // group
        {
            require!(
                self.group.load()?.is_ix_enabled(IxGate::PerpPlaceOrder),
                MangoError::IxIsDisabled
            );
        }

        // account
        {
            require!(self.account.as_ref().is_writable, ErrorCode::ConstraintMut);
            let account = self.account.load()?;
            require_keys_eq!(account.group, self.group.key());
            require!(account.is_operational(), MangoError::AccountIsFrozen);

            // account constraint #1
            require!(
                account.is_owner_or_delegate_or_self(owner, &self.account.key()),
                MangoError::SomeError
            );
        }

        // perp market
        {
            require!(
                self.perp_market.as_ref().is_writable,
                ErrorCode::ConstraintMut
            );
            let perp_market = self.perp_market.load()?;
            require_keys_eq!(perp_market.group, self.group.key());
            require_keys_eq!(perp_market.bids, self.bids.key());
            require_keys_eq!(perp_market.asks, self.asks.key());
            require_keys_eq!(perp_market.event_queue, self.event_queue.key());
            require_keys_eq!(perp_market.oracle, self.oracle.key());
        }

        // bids, asks, event_queue
        {
            require!(self.bids.as_ref().is_writable, ErrorCode::ConstraintMut);
            require!(self.asks.as_ref().is_writable, ErrorCode::ConstraintMut);
            require!(
                self.event_queue.as_ref().is_writable,
                ErrorCode::ConstraintMut
            );
        }

        Ok(())
    }
}
