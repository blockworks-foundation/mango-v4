use std::str::FromStr;

use anchor_lang::prelude::Pubkey;
use serde::{Deserialize, Serialize};

use anchor_lang::Id;
use anchor_spl::token::Token;

use crate::MangoClient;
use crate::{util, TransactionBuilder};

use anyhow::Context;
use solana_sdk::{instruction::Instruction, signature::Signature};

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
    pub token_ledger_instruction: Option<InstructionResponse>,
    pub compute_budget_instructions: Option<Vec<InstructionResponse>>,
    pub setup_instructions: Option<Vec<InstructionResponse>>,
    pub swap_instruction: InstructionResponse,
    pub cleanup_instructions: Option<Vec<InstructionResponse>>,
    pub address_lookup_table_addresses: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstructionResponse {
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

impl TryFrom<&InstructionResponse> for solana_sdk::instruction::Instruction {
    type Error = anyhow::Error;
    fn try_from(m: &InstructionResponse) -> Result<Self, Self::Error> {
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

pub struct JupiterV6<'a> {
    pub mango_client: &'a MangoClient,
}

impl<'a> JupiterV6<'a> {
    pub async fn quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        slippage_bps: u64,
        only_direct_routes: bool,
    ) -> anyhow::Result<QuoteResponse> {
        let mut account = self.mango_client.mango_account().await?;
        let input_token_index = self
            .mango_client
            .context
            .token_by_mint(&input_mint)?
            .token_index;
        let output_token_index = self
            .mango_client
            .context
            .token_by_mint(&output_mint)?
            .token_index;
        account.ensure_token_position(input_token_index)?;
        account.ensure_token_position(output_token_index)?;

        let health_account_num =
            // bank and oracle
            2 * account.active_token_positions().count()
            // perp market and oracle
            + 2 * account.active_perp_positions().count()
            // open orders account
            + account.active_serum3_orders().count();
        // The mango instructions need the health account plus
        // mango program and group and account and instruction introspection.
        // Other accounts are shared between jupiter and mango:
        // token accounts, mints, token program, ata program, owner
        let extra_accounts = 4;
        // To produce more of a margin for error (also for the tx bytes size)
        let buffer_accounts = 6;
        let flash_loan_account_num = health_account_num + extra_accounts + buffer_accounts;

        let mut query_args = vec![
            ("inputMint", input_mint.to_string()),
            ("outputMint", output_mint.to_string()),
            ("amount", format!("{}", amount)),
            ("slippageBps", format!("{}", slippage_bps)),
            ("onlyDirectRoutes", only_direct_routes.to_string()),
            (
                "maxAccounts",
                format!(
                    "{}",
                    crate::MAX_ACCOUNTS_PER_TRANSACTION - flash_loan_account_num
                ),
            ),
        ];
        let config = self.mango_client.client.config();
        if !config.jupiter_token.is_empty() {
            query_args.push(("token", config.jupiter_token.clone()));
        }

        let response = self
            .mango_client
            .http_client
            .get(format!("{}/quote", config.jupiter_v6_url))
            .query(&query_args)
            .send()
            .await
            .context("quote request to jupiter")?;
        let quote: QuoteResponse =
            util::http_error_handling(response).await.with_context(|| {
                format!("error requesting jupiter route between {input_mint} and {output_mint}")
            })?;

        Ok(quote)
    }

    /// Find the instructions and account lookup tables for a jupiter swap through mango
    pub async fn prepare_swap_transaction(
        &self,
        quote: &QuoteResponse,
    ) -> anyhow::Result<TransactionBuilder> {
        let input_mint = Pubkey::from_str(&quote.input_mint)?;
        let output_mint = Pubkey::from_str(&quote.output_mint)?;

        let source_token = self.mango_client.context.token_by_mint(&input_mint)?;
        let target_token = self.mango_client.context.token_by_mint(&output_mint)?;

        let bank_ams = [source_token.first_bank(), target_token.first_bank()]
            .into_iter()
            .map(util::to_writable_account_meta)
            .collect::<Vec<_>>();

        let vault_ams = [source_token.first_vault(), target_token.first_vault()]
            .into_iter()
            .map(util::to_writable_account_meta)
            .collect::<Vec<_>>();

        let owner = self.mango_client.owner();
        let account = &self.mango_client.mango_account().await?;

        let token_ams = [source_token.mint, target_token.mint]
            .into_iter()
            .map(|mint| {
                util::to_writable_account_meta(
                    anchor_spl::associated_token::get_associated_token_address(&owner, &mint),
                )
            })
            .collect::<Vec<_>>();

        let source_loan = quote
            .in_amount
            .as_ref()
            .map(|v| u64::from_str(v).unwrap())
            .unwrap_or(0);
        let loan_amounts = vec![source_loan, 0u64];
        let num_loans: u8 = loan_amounts.len().try_into().unwrap();

        // This relies on the fact that health account banks will be identical to the first_bank above!
        let (health_ams, _health_cu) = self
            .mango_client
            .derive_health_check_remaining_account_metas(
                account,
                vec![source_token.token_index, target_token.token_index],
                vec![source_token.token_index, target_token.token_index],
                vec![],
            )
            .await
            .context("building health accounts")?;

        let mut query_args = vec![];
        let config = self.mango_client.client.config();
        if !config.jupiter_token.is_empty() {
            query_args.push(("token", config.jupiter_token.clone()));
        }

        let swap_response = self
            .mango_client
            .http_client
            .post(format!("{}/swap-instructions", config.jupiter_v6_url))
            .query(&query_args)
            .json(&SwapRequest {
                user_public_key: owner.to_string(),
                wrap_and_unwrap_sol: false,
                use_shared_accounts: true,
                fee_account: None,
                compute_unit_price_micro_lamports: None, // we already prioritize
                as_legacy_transaction: false,
                use_token_ledger: false,
                destination_token_account: None, // default to user ata
                quote_response: quote.clone(),
            })
            .send()
            .await
            .context("swap transaction request to jupiter")?;

        let swap: SwapInstructionsResponse = util::http_error_handling(swap_response)
            .await
            .context("error requesting jupiter swap")?;

        let mut instructions: Vec<Instruction> = Vec::new();

        for ix in &swap.compute_budget_instructions.unwrap_or_default() {
            instructions.push(ix.try_into()?);
        }
        for ix in &swap.setup_instructions.unwrap_or_default() {
            instructions.push(ix.try_into()?);
        }

        // Ensure the source token account is created (jupiter takes care of the output account)
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &owner,
                &owner,
                &source_token.mint,
                &Token::id(),
            ),
        );

        instructions.push(Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::FlashLoanBegin {
                        account: self.mango_client.mango_account_address,
                        owner,
                        token_program: Token::id(),
                        instructions: solana_sdk::sysvar::instructions::id(),
                    },
                    None,
                );
                ams.extend(bank_ams);
                ams.extend(vault_ams.clone());
                ams.extend(token_ams.clone());
                ams.push(util::to_readonly_account_meta(self.mango_client.group()));
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::FlashLoanBegin {
                loan_amounts,
            }),
        });
        instructions.push((&swap.swap_instruction).try_into()?);
        instructions.push(Instruction {
            program_id: mango_v4::id(),
            accounts: {
                let mut ams = anchor_lang::ToAccountMetas::to_account_metas(
                    &mango_v4::accounts::FlashLoanEnd {
                        account: self.mango_client.mango_account_address,
                        owner,
                        token_program: Token::id(),
                    },
                    None,
                );
                ams.extend(health_ams);
                ams.extend(vault_ams);
                ams.extend(token_ams);
                ams.push(util::to_readonly_account_meta(self.mango_client.group()));
                ams
            },
            data: anchor_lang::InstructionData::data(&mango_v4::instruction::FlashLoanEndV2 {
                num_loans,
                flash_loan_type: mango_v4::accounts_ix::FlashLoanType::Swap,
            }),
        });
        for ix in &swap.cleanup_instructions.unwrap_or_default() {
            instructions.push(ix.try_into()?);
        }

        let mut address_lookup_tables = self.mango_client.mango_address_lookup_tables().await?;
        let jup_alt_addresses = swap
            .address_lookup_table_addresses
            .map(|list| {
                list.iter()
                    .map(|s| Pubkey::from_str(s))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or(Ok(vec![]))?;
        let jup_alts = self
            .mango_client
            .fetch_address_lookup_tables(jup_alt_addresses.iter())
            .await?;
        address_lookup_tables.extend(jup_alts.into_iter());

        let payer = owner; // maybe use fee_payer? but usually it's the same

        Ok(TransactionBuilder {
            instructions,
            address_lookup_tables,
            payer,
            signers: vec![self.mango_client.owner.clone()],
            config: self
                .mango_client
                .client
                .config()
                .transaction_builder_config
                .clone(),
        })
    }

    pub async fn swap(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        slippage_bps: u64,
        only_direct_routes: bool,
    ) -> anyhow::Result<Signature> {
        let route = self
            .quote(
                input_mint,
                output_mint,
                amount,
                slippage_bps,
                only_direct_routes,
            )
            .await?;

        let tx_builder = self.prepare_swap_transaction(&route).await?;

        tx_builder.send_and_confirm(&self.mango_client.client).await
    }
}
