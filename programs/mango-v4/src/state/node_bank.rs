use anchor_lang::prelude::*;
use fixed::types::I80F48;

pub struct NodeBank {
    pub meta_data: MetaData,

    pub deposits: I80F48,
    pub borrows: I80F48,
    pub vault: Pubkey,
}
