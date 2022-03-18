use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use arrayref::array_refs;
use borsh::{BorshDeserialize, BorshSerialize};
use num_enum::TryFromPrimitive;
use std::io::Write;
use std::num::NonZeroU64;

use anchor_spl::dex;
use dex::serum_dex;
use serum_dex::instruction::NewOrderInstructionV3;
use serum_dex::matching::Side;

use crate::error::*;
use crate::state::*;

/// Unfortunately NewOrderInstructionV3 isn't borsh serializable.
///
/// Make a newtype and implement the traits for it.
pub struct NewOrderInstructionData(pub serum_dex::instruction::NewOrderInstructionV3);

/// mango-v3's deserialization code
fn unpack_dex_new_order_v3(
    data: &[u8; 46],
) -> Option<serum_dex::instruction::NewOrderInstructionV3> {
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

    Some(serum_dex::instruction::NewOrderInstructionV3 {
        side,
        limit_price,
        max_coin_qty,
        max_native_pc_qty_including_fees,
        self_trade_behavior,
        order_type,
        client_order_id,
        limit,
    })
}

impl BorshDeserialize for NewOrderInstructionData {
    fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
        let data: &[u8; 46] = buf[0..46]
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e))?;
        *buf = &buf[46..];
        Ok(Self(unpack_dex_new_order_v3(data).ok_or(
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error!(MangoError::SomeError),
            ),
        )?))
    }
}

impl BorshSerialize for NewOrderInstructionData {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::result::Result<(), std::io::Error> {
        let d = &self.0;
        let side: u8 = d.side.into();
        // TODO: why use four bytes here? (also in deserialization above)
        writer.write(&(side as u32).to_le_bytes())?;
        writer.write(&u64::from(d.limit_price).to_le_bytes())?;
        writer.write(&u64::from(d.max_coin_qty).to_le_bytes())?;
        writer.write(&u64::from(d.max_native_pc_qty_including_fees).to_le_bytes())?;
        let self_trade_behavior: u8 = d.self_trade_behavior.into();
        writer.write(&(self_trade_behavior as u32).to_le_bytes())?;
        let order_type: u8 = d.order_type.into();
        writer.write(&(order_type as u32).to_le_bytes())?;
        writer.write(&u64::from(d.client_order_id).to_le_bytes())?;
        writer.write(&u16::from(d.limit).to_le_bytes())?;
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
    // to be in the remainingAccounts (where they need to be anyway, for
    // health checks - but they need to be mut)
    // Validated inline
    #[account(mut)]
    pub quote_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub base_bank: AccountLoader<'info, Bank>,
    #[account(mut)]
    pub base_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
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
                .ok_or(error!(MangoError::SomeError))?
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
    cpi_place_order(&ctx, order.0)?;
    cpi_settle_funds(&ctx)?;

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
        base_bank.change(base_position, (after_base_vault - before_base_vault) as i64)?;

        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        let quote_position = account.token_account_map.get_mut(quote_bank.token_index)?;
        quote_bank.change(
            quote_position,
            (after_quote_vault - before_quote_vault) as i64,
        )?;
    }

    //
    // Health check
    //
    let account = ctx.accounts.account.load()?;
    let health = compute_health(&account, &ctx.remaining_accounts)?;
    msg!("health: {}", health);
    require!(health >= 0, MangoError::SomeError);

    Ok(())
}

fn cpi_place_order(ctx: &Context<Serum3PlaceOrder>, order: NewOrderInstructionV3) -> Result<()> {
    let order_payer_token_account = match order.side {
        Side::Bid => &ctx.accounts.quote_vault,
        Side::Ask => &ctx.accounts.base_vault,
    };

    let data = serum_dex::instruction::MarketInstruction::NewOrderV3(order).pack();
    let instruction = solana_program::instruction::Instruction {
        program_id: *ctx.accounts.serum_program.key,
        data,
        accounts: vec![
            AccountMeta::new(*ctx.accounts.serum_market_external.key, false),
            AccountMeta::new(*ctx.accounts.open_orders.key, false),
            AccountMeta::new(*ctx.accounts.market_request_queue.key, false),
            AccountMeta::new(*ctx.accounts.market_event_queue.key, false),
            AccountMeta::new(*ctx.accounts.market_bids.key, false),
            AccountMeta::new(*ctx.accounts.market_asks.key, false),
            AccountMeta::new(order_payer_token_account.key(), false),
            AccountMeta::new_readonly(ctx.accounts.group.key(), true),
            AccountMeta::new(*ctx.accounts.market_base_vault.key, false),
            AccountMeta::new(*ctx.accounts.market_quote_vault.key, false),
            AccountMeta::new_readonly(*ctx.accounts.token_program.key, false),
            AccountMeta::new_readonly(ctx.accounts.group.key(), false),
        ],
    };
    let account_infos = [
        ctx.accounts.serum_program.to_account_info(), // Have to add account of the program id
        ctx.accounts.serum_market_external.to_account_info(),
        ctx.accounts.open_orders.to_account_info(),
        ctx.accounts.market_request_queue.to_account_info(),
        ctx.accounts.market_event_queue.to_account_info(),
        ctx.accounts.market_bids.to_account_info(),
        ctx.accounts.market_asks.to_account_info(),
        order_payer_token_account.to_account_info(),
        ctx.accounts.group.to_account_info(),
        ctx.accounts.market_base_vault.to_account_info(),
        ctx.accounts.market_quote_vault.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.group.to_account_info(),
    ];

    let group = ctx.accounts.group.load()?;
    let seeds = group_seeds!(group);
    solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

    Ok(())
}

fn cpi_settle_funds(ctx: &Context<Serum3PlaceOrder>) -> Result<()> {
    let data = serum_dex::instruction::MarketInstruction::SettleFunds.pack();
    let instruction = solana_program::instruction::Instruction {
        program_id: *ctx.accounts.serum_program.key,
        data,
        accounts: vec![
            AccountMeta::new(*ctx.accounts.serum_market_external.key, false),
            AccountMeta::new(*ctx.accounts.open_orders.key, false),
            AccountMeta::new_readonly(ctx.accounts.group.key(), true),
            AccountMeta::new(*ctx.accounts.market_base_vault.key, false),
            AccountMeta::new(*ctx.accounts.market_quote_vault.key, false),
            AccountMeta::new(ctx.accounts.base_vault.key(), false),
            AccountMeta::new(ctx.accounts.quote_vault.key(), false),
            AccountMeta::new_readonly(*ctx.accounts.market_vault_signer.key, false),
            AccountMeta::new_readonly(*ctx.accounts.token_program.key, false),
            AccountMeta::new(ctx.accounts.quote_vault.key(), false),
        ],
    };

    let account_infos = [
        ctx.accounts.serum_market_external.to_account_info(),
        ctx.accounts.serum_market_external.to_account_info(),
        ctx.accounts.open_orders.to_account_info(),
        ctx.accounts.group.to_account_info(),
        ctx.accounts.market_base_vault.to_account_info(),
        ctx.accounts.market_quote_vault.to_account_info(),
        ctx.accounts.base_vault.to_account_info(),
        ctx.accounts.quote_vault.to_account_info(),
        ctx.accounts.market_vault_signer.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.quote_vault.to_account_info(),
    ];

    let group = ctx.accounts.group.load()?;
    let seeds = group_seeds!(group);
    solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

    Ok(())
}
