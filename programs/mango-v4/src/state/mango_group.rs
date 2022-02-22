use anchor_lang::prelude::*;

const MAX_TOKENS: usize = 100;
// TODO: Assuming we allow up to 65536 different tokens
pub type TokenIndex = u16;

#[zero_copy] // is repr(packed) still a problem
#[derive(Default)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub decimals: u8,
    // TODO: store oracle index here?
    pub reserved: [u8; 31], // TODO: size?
                            // token's bank account is a PDA
}
// TODO: static assert the size and alignment

impl TokenInfo {
    pub fn is_valid(&self) -> bool {
        self.mint != Pubkey::default()
    }
}

// TODO: Should we call this `Group` instead of `MangoGroup`? And `Account` instead of `MangoAccount`?
#[account(zero_copy)]
pub struct MangoGroup {
    // Relying on Anchor's discriminator be sufficient for our versioning needs?
    // pub meta_data: MetaData,
    pub owner: Pubkey,

    //pub num_oracles: usize, // incremented every time add_oracle is called
    //pub oracles: [Pubkey; MAX_PAIRS],
    pub tokens: [TokenInfo; MAX_TOKENS],
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
}
// TODO: static assert the size and alignment

// Needs to be manual because large static arrays don't have Default
impl Default for MangoGroup {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            tokens: [TokenInfo::default(); MAX_TOKENS],
        }
    }
}
