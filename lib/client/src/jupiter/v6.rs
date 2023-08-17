use std::str::FromStr;

use anchor_lang::prelude::Pubkey;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub input_mint: String,
    pub in_amount: Option<String>,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: i32,
    pub platform_fee: Option<PlatformFee>,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlan>,
    pub context_slot: u64,
    pub time_taken: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlan {
    pub percent: i32,
    pub swap_info: Option<SwapInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: Option<String>,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub user_public_key: String,
    pub wrap_and_unwrap_sol: bool,
    pub use_shared_accounts: bool,
    pub fee_account: Option<String>,
    pub compute_unit_price_micro_lamports: Option<u64>,
    pub as_legacy_transaction: bool,
    pub use_token_ledger: bool,
    pub destination_token_account: Option<String>,
    pub quote_response: QuoteResponse,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub swap_transaction: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapInstructionsResponse {
    pub token_ledger_instruction: Option<Instruction>,
    pub compute_budget_instructions: Option<Vec<Instruction>>,
    pub setup_instructions: Option<Vec<Instruction>>,
    pub swap_instruction: Instruction,
    pub cleanup_instructions: Option<Vec<Instruction>>,
    pub address_lookup_table_addresses: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    pub program_id: String,
    pub data: Option<String>,
    pub accounts: Option<Vec<AccountMeta>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: Option<bool>,
    pub is_writable: Option<bool>,
}

impl TryFrom<&Instruction> for solana_sdk::instruction::Instruction {
    type Error = anyhow::Error;
    fn try_from(m: &Instruction) -> Result<Self, Self::Error> {
        Ok(Self {
            program_id: Pubkey::from_str(&m.program_id)?,
            data: m
                .data
                .as_ref()
                .map(|d| base64::decode(d))
                .unwrap_or(Ok(vec![]))?,
            accounts: m
                .accounts
                .as_ref()
                .map(|accs| {
                    accs.iter()
                        .map(|a| a.try_into())
                        .collect::<anyhow::Result<Vec<solana_sdk::instruction::AccountMeta>>>()
                })
                .unwrap_or(Ok(vec![]))?,
        })
    }
}

impl TryFrom<&AccountMeta> for solana_sdk::instruction::AccountMeta {
    type Error = anyhow::Error;
    fn try_from(m: &AccountMeta) -> Result<Self, Self::Error> {
        Ok(Self {
            pubkey: Pubkey::from_str(&m.pubkey)?,
            is_signer: m.is_signer.unwrap_or(false),
            is_writable: m.is_writable.unwrap_or(false),
        })
    }
}
