use crate::state::perp_market_info::PerpMarketInfo;
use crate::state::spot_market_info::SpotMarketInfo;
use anchor_lang::prelude::*;

pub struct MangoGroup {
    pub meta_data: MetaData,
    pub num_oracles: usize, // incremented every time add_oracle is called

    pub tokens: [TokenInfo; MAX_TOKENS],
    // todo: make a reference, keep state in this new account
    pub spot_markets: [SpotMarketInfo; MAX_PAIRS],
    // todo: make a reference, keep state in this new account
    pub perp_markets: [PerpMarketInfo; MAX_PAIRS],

    // todo: make a reference, keep state in this new account
    pub oracles: [Pubkey; MAX_PAIRS],

    pub signer_nonce: u64,
    pub signer_key: Pubkey,
    pub admin: Pubkey, // Used to add new markets and adjust risk params
    // todo: which more dex'es do we want to support? orca for pure swap?
    pub dex_program_id: Pubkey, // Consider allowing more,
    pub mango_cache: Pubkey,
    pub valid_interval: u64,

    // todo: do we need more vaults?
    // insurance vault is funded by the Mango DAO with USDC and can be withdrawn by the DAO
    pub insurance_vault: Pubkey,
    pub srm_vault: Pubkey,
    pub msrm_vault: Pubkey,
    pub fees_vault: Pubkey,

    // todo: do we need this limit? afaik this was for ts liquidator to keep on working, maybe
    // with liquidatable-accounts-feed we have some sort of scaling? maybe we bought
    // some more breathing space?
    pub max_mango_accounts: u32, // limits maximum number of MangoAccounts.v1 (closeable) accounts
    pub num_mango_accounts: u32, // number of MangoAccounts.v1

    pub ref_surcharge_centibps: u32, // 100
    pub ref_share_centibps: u32,     // 80 (must be less than surcharge)
    pub ref_mngo_required: u64,
    // todo: padding should be called reserve instead,
    // padding is misleading in context of zero copy
    pub padding: [u8; 8], // padding used for future expansions
}
