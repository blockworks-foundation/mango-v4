use anchor_lang::prelude::*;
use fixed::types::I80F48;

// todo: just call bank instead of rootbank?
pub struct RootBank {
    pub meta_data: MetaData,

    // todo: multi-leg interest
    pub optimal_util: I80F48,
    pub optimal_rate: I80F48,
    pub max_rate: I80F48,

    pub num_node_banks: usize,
    // todo: fold node bank into rootbank, the optimisation was never used
    pub node_banks: [Pubkey; MAX_NODE_BANKS],

    pub deposit_index: I80F48,
    pub borrow_index: I80F48,
    pub last_updated: u64,

    padding: [u8; 64], // used for future expansions
}
