use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use arrayref::array_refs;
use borsh::{BorshDeserialize, BorshSerialize};
use fixed::types::I80F48;
use num_enum::TryFromPrimitive;
use serum_dex::matching::Side;
use std::io::Write;
use std::num::NonZeroU64;

use crate::error::*;
use crate::state::*;

/// Unfortunately NewOrderInstructionV3 isn't borsh serializable.
///
/// Make a newtype and implement the traits for it.
pub struct NewOrderInstructionData(pub serum_dex::instruction::NewOrderInstructionV3);

impl NewOrderInstructionData {
    // Copy of NewOrderInstructionV3::unpack(), which we wish were public!
    fn unpack(data: &[u8; 46]) -> Option<Self> {
        let (
            &side_arr,
            &price_arr,
            &max_coin_qty_arr,
            &max_native_pc_qty_arr,
            &self_trade_behavior_arr,
            &otype_arr,
            &client_order_id_bytes,
            &limit_arr,
        ) = array_refs![data, 4, 8, 8, 8, 4, 4, 8, 2];

        let side = serum_dex::matching::Side::try_from_primitive(
            u32::from_le_bytes(side_arr).try_into().ok()?,
        )
        .ok()?;
        let limit_price = NonZeroU64::new(u64::from_le_bytes(price_arr))?;
        let max_coin_qty = NonZeroU64::new(u64::from_le_bytes(max_coin_qty_arr))?;
        let max_native_pc_qty_including_fees =
            NonZeroU64::new(u64::from_le_bytes(max_native_pc_qty_arr))?;
        let self_trade_behavior = serum_dex::instruction::SelfTradeBehavior::try_from_primitive(
            u32::from_le_bytes(self_trade_behavior_arr)
                .try_into()
                .ok()?,
        )
        .ok()?;
        let order_type = serum_dex::matching::OrderType::try_from_primitive(
            u32::from_le_bytes(otype_arr).try_into().ok()?,
        )
        .ok()?;
        let client_order_id = u64::from_le_bytes(client_order_id_bytes);
        let limit = u16::from_le_bytes(limit_arr);

        Some(Self(serum_dex::instruction::NewOrderInstructionV3 {
            side,
            limit_price,
            max_coin_qty,
            max_native_pc_qty_including_fees,
            self_trade_behavior,
            order_type,
            client_order_id,
            limit,
        }))
    }
}

impl BorshDeserialize for NewOrderInstructionData {
    fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
        let data: &[u8; 46] = buf[0..46]
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e))?;
        *buf = &buf[46..];
        Self::unpack(data).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error!(MangoError::SomeError),
            )
        })
    }
}

impl BorshSerialize for NewOrderInstructionData {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::result::Result<(), std::io::Error> {
        // serum_dex uses bincode::serialize() internally, see MarketInstruction::pack()
        writer.write_all(&bincode::serialize(&self.0).unwrap())?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Serum3PlaceOrder<'info> {
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
    #[account(mut)]
    pub market_request_queue: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_base_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub market_quote_vault: UncheckedAccount<'info>,
    // needed for the automatic settle_funds call
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

pub fn serum3_place_order(
    ctx: Context<Serum3PlaceOrder>,
    order: NewOrderInstructionData,
) -> Result<()> {
    //
    // Validation
    //
    {
        let account = ctx.accounts.account.load()?;
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

    // TODO: pre-health check

    //
    // Apply the order to serum. Also immediately settle, in case the order
    // matched against an existing other order.
    //
    cpi_place_order(ctx.accounts, order)?;
    cpi_settle_funds(ctx.accounts)?;

    //
    // After-order tracking
    //
    ctx.accounts.base_vault.reload()?;
    ctx.accounts.quote_vault.reload()?;
    let after_base_vault = ctx.accounts.base_vault.amount;
    let after_quote_vault = ctx.accounts.quote_vault.amount;

    // Charge the difference in vault balances to the user's account
    {
        let mut account = ctx.accounts.account.load_mut()?;

        let mut base_bank = ctx.accounts.base_bank.load_mut()?;
        let base_position = account.token_account_map.get_mut(base_bank.token_index)?;
        base_bank.change(
            base_position,
            I80F48::from(after_base_vault) - I80F48::from(before_base_vault),
        )?;

        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        let quote_position = account.token_account_map.get_mut(quote_bank.token_index)?;
        quote_bank.change(
            quote_position,
            I80F48::from(after_quote_vault) - I80F48::from(before_quote_vault),
        )?;
    }

    //
    // Health check
    //
    let account = ctx.accounts.account.load()?;
    let health = compute_health_from_fixed_accounts(&account, &ctx.remaining_accounts)?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::SomeError);

    Ok(())
}

fn cpi_place_order(ctx: &Serum3PlaceOrder, order: NewOrderInstructionData) -> Result<()> {
    use crate::serum3_cpi;

    let order_payer_token_account = match order.0.side {
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
    .call(&group, order.0)
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
