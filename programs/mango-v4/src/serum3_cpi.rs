use anchor_lang::prelude::*;
use serum_dex::state::{OpenOrders, ToAlignedBytes, ACCOUNT_HEAD_PADDING};

use std::cell::{Ref, RefMut};
use std::cmp::min;
use std::convert::identity;
use std::mem::size_of;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::*;

/// Serum padding is "serum" + data + "padding"
fn strip_dex_padding(data: &[u8]) -> Result<&[u8]> {
    require!(data.len() >= 12, MangoError::SomeError);
    Ok(&data[5..data.len() - 7])
}

fn strip_dex_padding_ref<'a>(acc: &'a AccountInfo) -> Result<Ref<'a, [u8]>> {
    require!(acc.data_len() >= 12, MangoError::SomeError);
    Ok(Ref::map(acc.try_borrow_data()?, |data| {
        &data[5..data.len() - 7]
    }))
}

fn strip_dex_padding_ref_mut<'a>(acc: &'a AccountInfo) -> Result<RefMut<'a, [u8]>> {
    require!(acc.data_len() >= 12, MangoError::SomeError);
    Ok(RefMut::map(acc.try_borrow_mut_data()?, |data| {
        let len = data.len();
        &mut data[5..len - 7]
    }))
}

#[inline]
pub fn remove_slop_mut<T: bytemuck::Pod>(bytes: &mut [u8]) -> &mut [T] {
    let slop = bytes.len() % size_of::<T>();
    let new_len = bytes.len() - slop;
    bytemuck::cast_slice_mut(&mut bytes[..new_len])
}

fn strip_data_header_mut<H: bytemuck::Pod, D: bytemuck::Pod>(
    orig_data: RefMut<[u8]>,
) -> Result<(RefMut<H>, RefMut<[D]>)> {
    Ok(RefMut::map_split(orig_data, |data| {
        let (header_bytes, inner_bytes) = data.split_at_mut(size_of::<H>());
        let header = bytemuck::try_from_bytes_mut(header_bytes).unwrap();
        let inner = remove_slop_mut(inner_bytes);
        (header, inner)
    }))
}

pub fn has_serum_header(data: &[u8]) -> bool {
    if data.len() < 5 {
        return false;
    }
    let head = &data[..5];
    head == ACCOUNT_HEAD_PADDING
}

pub fn load_market_state<'a>(
    market_account: &'a AccountInfo,
    program_id: &Pubkey,
) -> Result<Ref<'a, serum_dex::state::MarketState>> {
    require!(market_account.owner == program_id, MangoError::SomeError);
    let state: Ref<serum_dex::state::MarketState> =
        Ref::map(strip_dex_padding_ref(market_account)?, |data| {
            bytemuck::from_bytes(data)
        });
    state
        .check_flags(false)
        .map_err(|_| error!(MangoError::SomeError))?;
    Ok(state)
}

/// Copied over from serum dex
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C, packed)]
pub struct OrderBookStateHeader {
    pub account_flags: u64, // Initialized, (Bids or Asks)
}

pub fn load_bids_mut<'a>(
    sm: &serum_dex::state::MarketState,
    bids: &'a AccountInfo,
) -> Result<RefMut<'a, serum_dex::critbit::Slab>> {
    require!(
        bids.key.to_aligned_bytes() == identity(sm.bids),
        MangoError::SomeError
    );
    let orig_data = strip_dex_padding_ref_mut(bids)?;
    let (header, buf) = strip_data_header_mut::<OrderBookStateHeader, u8>(orig_data)?;
    require!(
        header.account_flags
            == serum_dex::state::AccountFlag::Initialized as u64
                + serum_dex::state::AccountFlag::Bids as u64,
        MangoError::SomeError
    );
    Ok(RefMut::map(buf, serum_dex::critbit::Slab::new))
}

pub fn load_asks_mut<'a>(
    sm: &serum_dex::state::MarketState,
    asks: &'a AccountInfo,
) -> Result<RefMut<'a, serum_dex::critbit::Slab>> {
    require!(
        asks.key.to_aligned_bytes() == identity(sm.asks),
        MangoError::SomeError
    );
    let orig_data = strip_dex_padding_ref_mut(asks)?;
    let (header, buf) = strip_data_header_mut::<OrderBookStateHeader, u8>(orig_data)?;
    require!(
        header.account_flags
            == serum_dex::state::AccountFlag::Initialized as u64
                + serum_dex::state::AccountFlag::Asks as u64,
        MangoError::SomeError
    );
    Ok(RefMut::map(buf, serum_dex::critbit::Slab::new))
}

pub fn load_open_orders_ref<'a>(
    acc: &'a AccountInfo,
) -> Result<Ref<'a, serum_dex::state::OpenOrders>> {
    Ok(Ref::map(strip_dex_padding_ref(acc)?, bytemuck::from_bytes))
}

pub fn load_open_orders(acc: &impl AccountReader) -> Result<&serum_dex::state::OpenOrders> {
    load_open_orders_bytes(acc.data())
}

pub fn load_open_orders_bytes(bytes: &[u8]) -> Result<&serum_dex::state::OpenOrders> {
    Ok(bytemuck::from_bytes(strip_dex_padding(bytes)?))
}

pub fn pubkey_from_u64_array(d: [u64; 4]) -> Pubkey {
    let b: [u8; 32] = bytemuck::cast(d);
    Pubkey::from(b)
}

/// For loan origination fees bookkeeping purposes
#[derive(Debug)]
pub struct OpenOrdersSlim {
    native_coin_free: u64,
    native_coin_total: u64,
    native_pc_free: u64,
    native_pc_total: u64,
    referrer_rebates_accrued: u64,
}
impl OpenOrdersSlim {
    pub fn from_oo(oo: &OpenOrders) -> Self {
        Self {
            native_coin_free: oo.native_coin_free,
            native_coin_total: oo.native_coin_total,
            native_pc_free: oo.native_pc_free,
            native_pc_total: oo.native_pc_total,
            referrer_rebates_accrued: oo.referrer_rebates_accrued,
        }
    }
}

pub trait OpenOrdersAmounts {
    fn native_base_reserved(&self) -> u64;
    fn native_quote_reserved(&self) -> u64;
    fn native_base_free(&self) -> u64;
    fn native_quote_free(&self) -> u64;
    fn native_base_total(&self) -> u64;
    fn native_quote_total(&self) -> u64;
    fn native_rebates(&self) -> u64;
}

impl OpenOrdersAmounts for OpenOrdersSlim {
    fn native_base_reserved(&self) -> u64 {
        self.native_coin_total - self.native_coin_free
    }
    fn native_quote_reserved(&self) -> u64 {
        self.native_pc_total - self.native_pc_free
    }
    fn native_base_free(&self) -> u64 {
        self.native_coin_free
    }
    fn native_quote_free(&self) -> u64 {
        self.native_pc_free
    }
    fn native_base_total(&self) -> u64 {
        self.native_coin_total
    }
    fn native_quote_total(&self) -> u64 {
        self.native_pc_total
    }
    fn native_rebates(&self) -> u64 {
        self.referrer_rebates_accrued
    }
}

impl OpenOrdersAmounts for OpenOrders {
    fn native_base_reserved(&self) -> u64 {
        self.native_coin_total - self.native_coin_free
    }
    fn native_quote_reserved(&self) -> u64 {
        self.native_pc_total - self.native_pc_free
    }
    fn native_base_free(&self) -> u64 {
        self.native_coin_free
    }
    fn native_quote_free(&self) -> u64 {
        self.native_pc_free
    }
    fn native_base_total(&self) -> u64 {
        self.native_coin_total
    }
    fn native_quote_total(&self) -> u64 {
        self.native_pc_total
    }
    fn native_rebates(&self) -> u64 {
        self.referrer_rebates_accrued
    }
}

pub struct InitOpenOrders<'info> {
    /// CHECK: cpi
    pub program: AccountInfo<'info>,
    /// CHECK: cpi
    pub market: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders_authority: AccountInfo<'info>,
    /// CHECK: cpi
    pub rent: AccountInfo<'info>,
}

impl<'info> InitOpenOrders<'info> {
    pub fn call(self, group: &Group) -> Result<()> {
        let data = serum_dex::instruction::MarketInstruction::InitOpenOrders.pack();
        let instruction = solana_program::instruction::Instruction {
            program_id: *self.program.key,
            data,
            accounts: vec![
                AccountMeta::new(*self.open_orders.key, false),
                AccountMeta::new_readonly(*self.open_orders_authority.key, true),
                AccountMeta::new_readonly(*self.market.key, false),
                AccountMeta::new_readonly(*self.rent.key, false),
            ],
        };

        let account_infos = [
            self.program,
            self.open_orders,
            self.open_orders_authority,
            self.market,
            self.rent,
        ];

        let seeds = group_seeds!(group);
        solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;
        Ok(())
    }
}

pub struct CloseOpenOrders<'info> {
    /// CHECK: cpi
    pub program: AccountInfo<'info>,
    /// CHECK: cpi
    pub market: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders_authority: AccountInfo<'info>,
    /// CHECK: cpi
    pub sol_destination: AccountInfo<'info>,
}

impl<'info> CloseOpenOrders<'info> {
    pub fn call(self, group: &Group) -> Result<()> {
        let data = serum_dex::instruction::MarketInstruction::CloseOpenOrders.pack();
        let instruction = solana_program::instruction::Instruction {
            program_id: *self.program.key,
            data,
            accounts: vec![
                AccountMeta::new(*self.open_orders.key, false),
                AccountMeta::new_readonly(*self.open_orders_authority.key, true),
                AccountMeta::new(*self.sol_destination.key, false),
                AccountMeta::new_readonly(*self.market.key, false),
            ],
        };

        let account_infos = [
            self.program,
            self.open_orders,
            self.open_orders_authority,
            self.sol_destination,
            self.market,
        ];

        let seeds = group_seeds!(group);
        solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;
        Ok(())
    }
}

pub struct SettleFunds<'info> {
    /// CHECK: cpi
    pub program: AccountInfo<'info>,
    /// CHECK: cpi
    pub market: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders_authority: AccountInfo<'info>,
    /// CHECK: cpi
    pub base_vault: AccountInfo<'info>,
    /// CHECK: cpi
    pub quote_vault: AccountInfo<'info>,
    /// CHECK: cpi
    pub user_base_wallet: AccountInfo<'info>,
    /// CHECK: cpi
    pub user_quote_wallet: AccountInfo<'info>,
    /// CHECK: cpi
    pub vault_signer: AccountInfo<'info>,
    /// CHECK: cpi
    pub token_program: AccountInfo<'info>,
    /// CHECK: cpi
    pub rebates_quote_wallet: AccountInfo<'info>,
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
                AccountMeta::new(*self.rebates_quote_wallet.key, false),
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
            self.user_quote_wallet,
            self.vault_signer,
            self.token_program,
            self.rebates_quote_wallet,
        ];

        let seeds = group_seeds!(group);
        solana_program::program::invoke_signed_unchecked(&instruction, &account_infos, &[seeds])?;

        Ok(())
    }
}

pub struct PlaceOrder<'info> {
    /// CHECK: cpi
    pub program: AccountInfo<'info>,
    /// CHECK: cpi
    pub market: AccountInfo<'info>,
    /// CHECK: cpi
    pub request_queue: AccountInfo<'info>,
    /// CHECK: cpi
    pub event_queue: AccountInfo<'info>,
    /// CHECK: cpi
    pub bids: AccountInfo<'info>,
    /// CHECK: cpi
    pub asks: AccountInfo<'info>,
    /// CHECK: cpi
    pub base_vault: AccountInfo<'info>,
    /// CHECK: cpi
    pub quote_vault: AccountInfo<'info>,
    /// CHECK: cpi
    pub token_program: AccountInfo<'info>,

    /// CHECK: cpi
    pub open_orders: AccountInfo<'info>,
    /// CHECK: cpi
    pub order_payer_token_account: AccountInfo<'info>,
    /// must cover the open_orders and the order_payer_token_account
    /// CHECK: cpi
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
    /// CHECK: cpi
    pub program: AccountInfo<'info>,
    /// CHECK: cpi
    pub market: AccountInfo<'info>,
    /// CHECK: cpi
    pub event_queue: AccountInfo<'info>,
    /// CHECK: cpi
    pub bids: AccountInfo<'info>,
    /// CHECK: cpi
    pub asks: AccountInfo<'info>,

    /// CHECK: cpi
    pub open_orders: AccountInfo<'info>,
    /// CHECK: cpi
    pub open_orders_authority: AccountInfo<'info>,
}

impl<'a> CancelOrder<'a> {
    pub fn cancel_one(
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

    pub fn cancel_one_by_client_order_id(self, group: &Group, client_order_id: u64) -> Result<()> {
        let data =
            serum_dex::instruction::MarketInstruction::CancelOrderByClientIdV2(client_order_id)
                .pack();
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

    pub fn cancel_all(self, group: &Group, mut limit: u8) -> Result<()> {
        // find all cancels by scanning open_orders/bids/asks
        let mut cancels = vec![];
        {
            let open_orders = load_open_orders_ref(&self.open_orders)?;
            let market = load_market_state(&self.market, self.program.key)?;
            let bids = load_bids_mut(&market, &self.bids)?;
            let asks = load_asks_mut(&market, &self.asks)?;

            limit = min(limit, open_orders.free_slot_bits.count_zeros() as u8);
            if limit == 0 {
                return Ok(());
            }
            for j in 0..128 {
                let slot_mask = 1u128 << j;
                if open_orders.free_slot_bits & slot_mask != 0 {
                    // means slot is free
                    continue;
                }
                let order_id = open_orders.orders[j];

                // free_slot_bits is only updated when the event queue is processed,
                // that means we need to scan bids/asks to see if the order is still alive
                let side = if open_orders.is_bid_bits & slot_mask != 0 {
                    match bids.find_by_key(order_id) {
                        None => continue,
                        Some(_) => serum_dex::matching::Side::Bid,
                    }
                } else {
                    match asks.find_by_key(order_id) {
                        None => continue,
                        Some(_) => serum_dex::matching::Side::Ask,
                    }
                };

                let cancel_instruction =
                    serum_dex::instruction::CancelOrderInstructionV2 { side, order_id };

                cancels.push(cancel_instruction);

                limit -= 1;
                if limit == 0 {
                    break;
                }
            }
        }

        let mut instruction = solana_program::instruction::Instruction {
            program_id: *self.program.key,
            data: vec![],
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

        for cancel in cancels.into_iter() {
            instruction.data =
                serum_dex::instruction::MarketInstruction::CancelOrderV2(cancel).pack();
            solana_program::program::invoke_signed_unchecked(
                &instruction,
                &account_infos,
                &[seeds],
            )?;
        }

        Ok(())
    }
}
