use crate::state::perp_market_info::PerpMarketInfo;
use crate::state::spot_market_info::SpotMarketInfo;
use anchor_lang::prelude::*;

pub struct MangoGroup {
    pub meta_data: MetaData,
    pub num_oracles: usize, // incremented every time add_oracle is called

    pub tokens: [TokenInfo; MAX_TOKENS],
    pub spot_markets: [SpotMarketInfo; MAX_PAIRS],
    pub perp_markets: [PerpMarketInfo; MAX_PAIRS],

    pub oracles: [Pubkey; MAX_PAIRS],

    pub signer_nonce: u64,
    pub signer_key: Pubkey,
    pub admin: Pubkey,          // Used to add new markets and adjust risk params
    pub dex_program_id: Pubkey, // Consider allowing more
    pub mango_cache: Pubkey,
    pub valid_interval: u64,

    // insurance vault is funded by the Mango DAO with USDC and can be withdrawn by the DAO
    pub insurance_vault: Pubkey,
    pub srm_vault: Pubkey,
    pub msrm_vault: Pubkey,
    pub fees_vault: Pubkey,

    pub max_mango_accounts: u32, // limits maximum number of MangoAccounts.v1 (closeable) accounts
    pub num_mango_accounts: u32, // number of MangoAccounts.v1

    pub ref_surcharge_centibps: u32, // 100
    pub ref_share_centibps: u32,     // 80 (must be less than surcharge)
    pub ref_mngo_required: u64,
    pub padding: [u8; 8], // padding used for future expansions
}
