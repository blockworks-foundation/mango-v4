use crate::state::perp_account::PerpAccount;
use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub struct MangoAccount {
    pub meta_data: MetaData,

    pub mango_group: Pubkey,
    pub owner: Pubkey,

    pub in_margin_basket: [bool; MAX_PAIRS],
    pub num_in_margin_basket: u8,

    // Spot and Margin related data
    // todo: maybe we want to split collateral and lending?
    // todo: see https://github.com/blockworks-foundation/mango-v4/issues/1
    // todo: how does ftx do this?
    // todo: make a reference, keep state in this new account
    pub deposits: [I80F48; MAX_TOKENS],
    // todo: make a reference, keep state in this new account
    pub borrows: [I80F48; MAX_TOKENS],
    // todo: make a reference, keep state in this new account
    pub spot_open_orders: [Pubkey; MAX_PAIRS],

    // Perps related data
    // todo: make a reference, keep state in this new account
    pub perp_accounts: [PerpAccount; MAX_PAIRS],

    pub order_market: [u8; MAX_PERP_OPEN_ORDERS],
    pub order_side: [Side; MAX_PERP_OPEN_ORDERS],
    pub orders: [i128; MAX_PERP_OPEN_ORDERS],
    pub client_order_ids: [u64; MAX_PERP_OPEN_ORDERS],

    pub msrm_amount: u64,

    /// This account cannot open new positions or borrow until `init_health >= 0`
    pub being_liquidated: bool,

    /// This account cannot do anything except go through `resolve_bankruptcy`
    pub is_bankrupt: bool,
    pub info: [u8; INFO_LEN],

    /// Starts off as zero pubkey and points to the AdvancedOrders account
    pub advanced_orders_key: Pubkey,

    /// Can this account be upgraded to v1 so it can be closed
    pub not_upgradable: bool,

    // Alternative authority/signer of transactions for a mango account
    pub delegate: Pubkey,

    /// padding for expansions
    /// Note: future expansion can also be just done via isolated PDAs
    /// which can be computed independently and dont need to be linked from
    /// this account
    pub padding: [u8; 5],
}
