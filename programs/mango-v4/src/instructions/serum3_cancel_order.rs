use anchor_lang::prelude::*;
use arrayref::array_refs;
use borsh::{BorshDeserialize, BorshSerialize};
use num_enum::TryFromPrimitive;
use std::io::Write;

use serum_dex::matching::Side;

use crate::error::*;
use crate::state::*;

/// Unfortunately CancelOrderInstructionV2 isn't borsh serializable.
///
/// Make a newtype and implement the traits for it.
pub struct CancelOrderInstructionData(pub serum_dex::instruction::CancelOrderInstructionV2);

impl CancelOrderInstructionData {
    // Copy of CancelOrderInstructionV2::unpack(), which we wish were public!
    fn unpack(data: &[u8; 20]) -> Option<Self> {
        let (&side_arr, &oid_arr) = array_refs![data, 4, 16];
        let side = Side::try_from_primitive(u32::from_le_bytes(side_arr).try_into().ok()?).ok()?;
        let order_id = u128::from_le_bytes(oid_arr);
        Some(Self(serum_dex::instruction::CancelOrderInstructionV2 {
            side,
            order_id,
        }))
    }
}

impl BorshDeserialize for CancelOrderInstructionData {
    fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
        let data: &[u8; 20] = buf[0..20]
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e))?;
        *buf = &buf[20..];
        Self::unpack(data).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error!(MangoError::SomeError),
            )
        })
    }
}

impl BorshSerialize for CancelOrderInstructionData {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::result::Result<(), std::io::Error> {
        // serum_dex uses bincode::serialize() internally, see MarketInstruction::pack()
        writer.write_all(&bincode::serialize(&self.0).unwrap())?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Serum3CancelOrder<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
        has_one = owner,
    )]
    pub account: AccountLoader<'info, MangoAccount>,
    pub owner: Signer<'info>,

    // Validated inline
    #[account(mut)]
    pub open_orders: UncheckedAccount<'info>,

    #[account(
        has_one = group,
        has_one = serum_program,
        has_one = serum_market_external,
    )]
    pub serum_market: AccountLoader<'info, Serum3Market>,
    pub serum_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub serum_market_external: UncheckedAccount<'info>,

    // These accounts are forwarded directly to the serum cpi call
    // and are validated there.
    #[account(mut)]
    pub market_bids: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_asks: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_event_queue: UncheckedAccount<'info>,
}

pub fn serum3_cancel_order(
    ctx: Context<Serum3CancelOrder>,
    order: CancelOrderInstructionData,
) -> Result<()> {
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
        require!(!account.is_bankrupt, MangoError::IsBankrupt);

        let serum_market = ctx.accounts.serum_market.load()?;

        // Validate open_orders
        require!(
            account
                .serum3_account_map
                .find(serum_market.market_index)
                .ok_or_else(|| error!(MangoError::SomeError))?
                .open_orders
                == ctx.accounts.open_orders.key(),
            MangoError::SomeError
        );
    }

    //
    // Cancel
    //
    cpi_cancel_order(ctx.accounts, order)?;

    Ok(())
}

fn cpi_cancel_order(ctx: &Serum3CancelOrder, order: CancelOrderInstructionData) -> Result<()> {
    use crate::serum3_cpi;
    let group = ctx.group.load()?;
    serum3_cpi::CancelOrder {
        program: ctx.serum_program.to_account_info(),
        market: ctx.serum_market_external.to_account_info(),
        bids: ctx.market_bids.to_account_info(),
        asks: ctx.market_asks.to_account_info(),
        event_queue: ctx.market_event_queue.to_account_info(),

        open_orders: ctx.open_orders.to_account_info(),
        open_orders_authority: ctx.group.to_account_info(),
    }
    .cancel_one(&group, order.0)
}
