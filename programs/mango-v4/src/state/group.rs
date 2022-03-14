use anchor_lang::prelude::*;

// TODO: Assuming we allow up to 65536 different tokens
pub type TokenIndex = u16;

// TODO: Should we call this `Group` instead of `Group`? And `Account` instead of `MangoAccount`?
#[account(zero_copy)]
pub struct Group {
    // Relying on Anchor's discriminator be sufficient for our versioning needs?
    // pub meta_data: MetaData,
    pub admin: Pubkey,

    //pub num_oracles: usize, // incremented every time add_oracle is called
    //pub oracles: [Pubkey; MAX_PAIRS],

    //pub spot_markets: [SpotMarketInfo; MAX_PAIRS],
    //pub perp_markets: [PerpMarketInfo; MAX_PAIRS],

    //pub signer_nonce: u64,
    //pub signer_key: Pubkey,
    // todo: which more dex'es do we want to support? orca for pure swap?
    //pub dex_program_id: Pubkey,

    //pub cache_valid_interval: u64,

    // todo: do we need this limit? afaik this was for ts liquidator to keep on working, maybe
    // with liquidatable-accounts-feed we have some sort of scaling? maybe we bought
    // some more breathing space?
    //pub max_mango_accounts: u32, // limits maximum number of MangoAccounts.v1 (closeable) accounts
    //pub num_mango_accounts: u32, // number of MangoAccounts.v1

    //pub ref_surcharge_centibps: u32, // 100
    //pub ref_share_centibps: u32,     // 80 (must be less than surcharge)
    //pub ref_mngo_required: u64,
    pub bump: u8,
}
// TODO: static assert the size and alignment

#[macro_export]
macro_rules! group_seeds {
    ( $group:expr ) => {
        &[b"Group".as_ref(), $group.admin.as_ref(), &[$group.bump]]
    };
}

pub use group_seeds;
