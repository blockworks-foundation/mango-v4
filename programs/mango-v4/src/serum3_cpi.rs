use anchor_lang::prelude::*;
use anchor_spl::dex::serum_dex;

use crate::state::*;

pub struct SettleFunds<'info> {
    pub program: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub open_orders: AccountInfo<'info>,
    pub open_orders_authority: AccountInfo<'info>,
    pub base_vault: AccountInfo<'info>,
    pub quote_vault: AccountInfo<'info>,
    pub user_base_wallet: AccountInfo<'info>,
    pub user_quote_wallet: AccountInfo<'info>,
    pub vault_signer: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

pub fn settle_funds(group: &Group, ctx: SettleFunds) -> Result<()> {
    let data = serum_dex::instruction::MarketInstruction::SettleFunds.pack();
    let instruction = solana_program::instruction::Instruction {
        program_id: *ctx.program.key,
        data,
        accounts: vec![
            AccountMeta::new(*ctx.market.key, false),
            AccountMeta::new(*ctx.open_orders.key, false),
            AccountMeta::new_readonly(*ctx.open_orders_authority.key, true),
            AccountMeta::new(*ctx.base_vault.key, false),
            AccountMeta::new(*ctx.quote_vault.key, false),
            AccountMeta::new(*ctx.user_base_wallet.key, false),
            AccountMeta::new(*ctx.user_quote_wallet.key, false),
            AccountMeta::new_readonly(*ctx.vault_signer.key, false),
            AccountMeta::new_readonly(*ctx.token_program.key, false),
            AccountMeta::new(*ctx.user_quote_wallet.key, false),
        ],
    };

    let account_infos = [
        ctx.program,
        ctx.market,
        ctx.open_orders,
        ctx.open_orders_authority,
        ctx.base_vault,
        ctx.quote_vault,
        ctx.user_base_wallet,
        ctx.user_quote_wallet.clone(),
        ctx.vault_signer,
        ctx.token_program,
        ctx.user_quote_wallet,
    ];

    let seeds = group_seeds!(group);
    solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

    Ok(())
}

pub struct PlaceOrder<'info> {
    pub program: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub request_queue: AccountInfo<'info>,
    pub event_queue: AccountInfo<'info>,
    pub bids: AccountInfo<'info>,
    pub asks: AccountInfo<'info>,
    pub base_vault: AccountInfo<'info>,
    pub quote_vault: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,

    pub open_orders: AccountInfo<'info>,
    pub order_payer_token_account: AccountInfo<'info>,
    // must cover the open_orders and the order_payer_token_account
    pub user_authority: AccountInfo<'info>,
}

pub fn place_order(
    group: &Group,
    ctx: PlaceOrder,
    order: serum_dex::instruction::NewOrderInstructionV3,
) -> Result<()> {
    let data = serum_dex::instruction::MarketInstruction::NewOrderV3(order).pack();
    let instruction = solana_program::instruction::Instruction {
        program_id: *ctx.program.key,
        data,
        accounts: vec![
            AccountMeta::new(*ctx.market.key, false),
            AccountMeta::new(*ctx.open_orders.key, false),
            AccountMeta::new(*ctx.request_queue.key, false),
            AccountMeta::new(*ctx.event_queue.key, false),
            AccountMeta::new(*ctx.bids.key, false),
            AccountMeta::new(*ctx.asks.key, false),
            AccountMeta::new(*ctx.order_payer_token_account.key, false),
            AccountMeta::new_readonly(*ctx.user_authority.key, true),
            AccountMeta::new(*ctx.base_vault.key, false),
            AccountMeta::new(*ctx.quote_vault.key, false),
            AccountMeta::new_readonly(*ctx.token_program.key, false),
            AccountMeta::new_readonly(*ctx.user_authority.key, false),
        ],
    };
    let account_infos = [
        ctx.program,
        ctx.market,
        ctx.open_orders,
        ctx.request_queue,
        ctx.event_queue,
        ctx.bids,
        ctx.asks,
        ctx.order_payer_token_account,
        ctx.user_authority.clone(),
        ctx.base_vault,
        ctx.quote_vault,
        ctx.token_program,
        ctx.user_authority,
    ];

    let seeds = group_seeds!(group);
    solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

    Ok(())
}
