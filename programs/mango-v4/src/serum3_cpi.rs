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

impl<'a> SettleFunds<'a> {
    pub fn call(self, group: &Group) -> Result<()> {
        let data = serum_dex::instruction::MarketInstruction::SettleFunds.pack();
        let instruction = solana_program::instruction::Instruction {
            program_id: *self.program.key,
            data,
            accounts: vec![
                AccountMeta::new(*self.market.key, false),
                AccountMeta::new(*self.open_orders.key, false),
                AccountMeta::new_readonly(*self.open_orders_authority.key, true),
                AccountMeta::new(*self.base_vault.key, false),
                AccountMeta::new(*self.quote_vault.key, false),
                AccountMeta::new(*self.user_base_wallet.key, false),
                AccountMeta::new(*self.user_quote_wallet.key, false),
                AccountMeta::new_readonly(*self.vault_signer.key, false),
                AccountMeta::new_readonly(*self.token_program.key, false),
                AccountMeta::new(*self.user_quote_wallet.key, false),
            ],
        };

        let account_infos = [
            self.program,
            self.market,
            self.open_orders,
            self.open_orders_authority,
            self.base_vault,
            self.quote_vault,
            self.user_base_wallet,
            self.user_quote_wallet.clone(),
            self.vault_signer,
            self.token_program,
            self.user_quote_wallet,
        ];

        let seeds = group_seeds!(group);
        solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

        Ok(())
    }
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

impl<'a> PlaceOrder<'a> {
    pub fn call(
        self,
        group: &Group,
        order: serum_dex::instruction::NewOrderInstructionV3,
    ) -> Result<()> {
        let data = serum_dex::instruction::MarketInstruction::NewOrderV3(order).pack();
        let instruction = solana_program::instruction::Instruction {
            program_id: *self.program.key,
            data,
            accounts: vec![
                AccountMeta::new(*self.market.key, false),
                AccountMeta::new(*self.open_orders.key, false),
                AccountMeta::new(*self.request_queue.key, false),
                AccountMeta::new(*self.event_queue.key, false),
                AccountMeta::new(*self.bids.key, false),
                AccountMeta::new(*self.asks.key, false),
                AccountMeta::new(*self.order_payer_token_account.key, false),
                AccountMeta::new_readonly(*self.user_authority.key, true),
                AccountMeta::new(*self.base_vault.key, false),
                AccountMeta::new(*self.quote_vault.key, false),
                AccountMeta::new_readonly(*self.token_program.key, false),
                AccountMeta::new_readonly(*self.user_authority.key, false),
            ],
        };
        let account_infos = [
            self.program,
            self.market,
            self.open_orders,
            self.request_queue,
            self.event_queue,
            self.bids,
            self.asks,
            self.order_payer_token_account,
            self.user_authority.clone(),
            self.base_vault,
            self.quote_vault,
            self.token_program,
            self.user_authority,
        ];

        let seeds = group_seeds!(group);
        solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

        Ok(())
    }
}

pub struct CancelOrder<'info> {
    pub program: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub event_queue: AccountInfo<'info>,
    pub bids: AccountInfo<'info>,
    pub asks: AccountInfo<'info>,

    pub open_orders: AccountInfo<'info>,
    pub open_orders_authority: AccountInfo<'info>,
}

impl<'a> CancelOrder<'a> {
    pub fn call(
        self,
        group: &Group,
        order: serum_dex::instruction::CancelOrderInstructionV2,
    ) -> Result<()> {
        let data = serum_dex::instruction::MarketInstruction::CancelOrderV2(order).pack();
        let instruction = solana_program::instruction::Instruction {
            program_id: *self.program.key,
            data,
            accounts: vec![
                AccountMeta::new(*self.market.key, false),
                AccountMeta::new(*self.bids.key, false),
                AccountMeta::new(*self.asks.key, false),
                AccountMeta::new(*self.open_orders.key, false),
                AccountMeta::new_readonly(*self.open_orders_authority.key, true),
                AccountMeta::new(*self.event_queue.key, false),
            ],
        };
        let account_infos = [
            self.program,
            self.market,
            self.bids,
            self.asks,
            self.open_orders,
            self.open_orders_authority,
            self.event_queue,
        ];

        let seeds = group_seeds!(group);
        solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

        Ok(())
    }
}
