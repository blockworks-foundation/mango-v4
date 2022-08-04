use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub data: Vec<QueryRoute>,
    pub time_taken: f64,
    pub context_slot: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryRoute {
    pub in_amount: u64,
    pub out_amount: u64,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub out_amount_with_slippage: u64,
    pub swap_mode: String,
    pub price_impact_pct: f64,
    pub market_infos: Vec<QueryMarketInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryMarketInfo {
    pub id: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub lp_fee: QueryFee,
    pub platform_fee: QueryFee,
    pub not_enough_liquidity: bool,
    pub price_impact_pct: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryFee {
    pub amount: u64,
    pub mint: String,
    pub pct: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub route: QueryRoute,
    pub user_public_key: String,
    pub wrap_unwrap_sol: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub setup_transaction: Option<String>,
    pub swap_transaction: String,
    pub cleanup_transaction: Option<String>,
}
