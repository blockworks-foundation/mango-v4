#![cfg(test)]

use anchor_lang::prelude::*;
use fixed::types::I80F48;
use serum_dex::state::OpenOrders;
use std::cell::RefCell;
use std::mem::size_of;
use std::rc::Rc;

use crate::state::*;

pub const DUMMY_NOW_TS: u64 = 0;
pub const DUMMY_PRICE: I80F48 = I80F48::ZERO;

// Implementing TestAccount directly for ZeroCopy + Owner leads to a conflict
// because OpenOrders may add impls for those in the future.
pub trait MyZeroCopy: anchor_lang::ZeroCopy + Owner {}
impl MyZeroCopy for StubOracle {}
impl MyZeroCopy for Bank {}
impl MyZeroCopy for PerpMarket {}

#[derive(Clone)]
pub struct TestAccount<T> {
    pub bytes: Vec<u8>,
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    pub lamports: u64,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TestAccount<T> {
    pub fn new(bytes: Vec<u8>, owner: Pubkey) -> Self {
        Self {
            bytes,
            owner,
            pubkey: Pubkey::new_unique(),
            lamports: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn as_account_info(&mut self) -> AccountInfo {
        AccountInfo {
            key: &self.pubkey,
            owner: &self.owner,
            lamports: Rc::new(RefCell::new(&mut self.lamports)),
            data: Rc::new(RefCell::new(&mut self.bytes)),
            is_signer: false,
            is_writable: false,
            executable: false,
            rent_epoch: 0,
        }
    }
}

impl<T: MyZeroCopy> TestAccount<T> {
    pub fn new_zeroed() -> Self {
        let mut bytes = vec![0u8; 8 + size_of::<T>()];
        bytes[0..8].copy_from_slice(&T::discriminator());
        Self::new(bytes, T::owner())
    }

    pub fn data(&mut self) -> &mut T {
        bytemuck::from_bytes_mut(&mut self.bytes[8..])
    }
}

impl TestAccount<OpenOrders> {
    pub fn new_zeroed() -> Self {
        let mut bytes = vec![0u8; 12 + size_of::<OpenOrders>()];
        bytes[0..5].copy_from_slice(b"serum");
        Self::new(bytes, Pubkey::new_unique())
    }

    pub fn data(&mut self) -> &mut OpenOrders {
        bytemuck::from_bytes_mut(&mut self.bytes[5..5 + size_of::<OpenOrders>()])
    }
}

pub fn mock_bank_and_oracle(
    group: Pubkey,
    token_index: TokenIndex,
    price: f64,
    init_weights: f64,
    maint_weights: f64,
) -> (TestAccount<Bank>, TestAccount<StubOracle>) {
    let mut oracle = TestAccount::<StubOracle>::new_zeroed();
    oracle.data().price = I80F48::from_num(price);
    let mut bank = TestAccount::<Bank>::new_zeroed();
    bank.data().token_index = token_index;
    bank.data().group = group;
    bank.data().oracle = oracle.pubkey;
    bank.data().deposit_index = I80F48::from(1_000_000);
    bank.data().borrow_index = I80F48::from(1_000_000);
    bank.data().init_asset_weight = I80F48::from_num(1.0 - init_weights);
    bank.data().init_liab_weight = I80F48::from_num(1.0 + init_weights);
    bank.data().maint_asset_weight = I80F48::from_num(1.0 - maint_weights);
    bank.data().maint_liab_weight = I80F48::from_num(1.0 + maint_weights);
    bank.data().stable_price_model.reset_to_price(price, 0);
    bank.data().deposit_weight_scale_start_quote = f64::MAX;
    bank.data().borrow_weight_scale_start_quote = f64::MAX;
    bank.data().net_borrow_limit_window_size_ts = 1; // dummy
    bank.data().net_borrow_limit_per_window_quote = i64::MAX; // max since we don't want this to interfere
    (bank, oracle)
}

pub fn mock_perp_market(
    group: Pubkey,
    oracle: Pubkey,
    price: f64,
    market_index: PerpMarketIndex,
    base_weights: (f64, f64),
    pnl_weights: (f64, f64),
) -> TestAccount<PerpMarket> {
    let mut pm = TestAccount::<PerpMarket>::new_zeroed();
    pm.data().group = group;
    pm.data().oracle = oracle;
    pm.data().perp_market_index = market_index;
    pm.data().init_base_asset_weight = I80F48::from_num(1.0 - base_weights.0);
    pm.data().init_base_liab_weight = I80F48::from_num(1.0 + base_weights.0);
    pm.data().maint_base_asset_weight = I80F48::from_num(1.0 - base_weights.1);
    pm.data().maint_base_liab_weight = I80F48::from_num(1.0 + base_weights.1);
    pm.data().init_overall_asset_weight = I80F48::from_num(1.0 - pnl_weights.0);
    pm.data().maint_overall_asset_weight = I80F48::from_num(1.0 - pnl_weights.1);
    pm.data().quote_lot_size = 100;
    pm.data().base_lot_size = 10;
    pm.data().stable_price_model.reset_to_price(price, 0);
    pm.data().settle_pnl_limit_window_size_ts = 1;
    pm
}
