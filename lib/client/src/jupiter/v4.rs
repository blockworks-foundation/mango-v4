use serde::{Deserialize, Serialize};
use std::str::FromStr;

use anchor_lang::Id;
use anchor_spl::token::Token;

use bincode::Options;

use crate::{util, TransactionBuilder};
use crate::{JupiterSwapMode, MangoClient};

use anyhow::Context;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Signature;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub data: Vec<QueryRoute>,
    pub time_taken: f64,
    pub context_slot: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryRoute {
    pub in_amount: String,
    pub out_amount: String,
    pub price_impact_pct: f64,
    pub market_infos: Vec<QueryMarketInfo>,
    pub amount: String,
    pub slippage_bps: u64,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub fees: Option<QueryRouteFees>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryMarketInfo {
    pub id: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub not_enough_liquidity: bool,
    pub in_amount: String,
    pub out_amount: String,
    pub min_in_amount: Option<String>,
    pub min_out_amount: Option<String>,
    pub price_impact_pct: Option<f64>,
    pub lp_fee: QueryFee,
    pub platform_fee: QueryFee,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryFee {
    pub amount: String,
    pub mint: String,
    pub pct: Option<f64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryRouteFees {
    pub signature_fee: f64,
    pub open_orders_deposits: Vec<f64>,
    pub ata_deposits: Vec<f64>,
    pub total_fee_and_deposits: f64,
    #[serde(rename = "minimalSOLForTransaction")]
    pub minimal_sol_for_transaction: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub route: QueryRoute,
    pub user_public_key: String,
    #[serde(rename = "wrapUnwrapSOL")]
    pub wrap_unwrap_sol: bool,
    pub compute_unit_price_micro_lamports: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub setup_transaction: Option<String>,
    pub swap_transaction: String,
    pub cleanup_transaction: Option<String>,
}

pub struct JupiterV4<'a> {
    pub mango_client: &'a MangoClient,
}

impl<'a> JupiterV4<'a> {
    pub async fn quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        slippage_bps: u64,
        swap_mode: JupiterSwapMode,
        only_direct_routes: bool,
    ) -> anyhow::Result<QueryRoute> {
        let response = self
            .mango_client
            .http_client
            .get("https://quote-api.jup.ag/v4/quote")
            .query(&[
                ("inputMint", input_mint.to_string()),
                ("outputMint", output_mint.to_string()),
                ("amount", format!("{}", amount)),
                ("onlyDirectRoutes", only_direct_routes.to_string()),
                ("enforceSingleTx", "true".into()),
                ("filterTopNResult", "10".into()),
                ("slippageBps", format!("{}", slippage_bps)),
                (
                    "swapMode",
                    match swap_mode {
                        JupiterSwapMode::ExactIn => "ExactIn",
                        JupiterSwapMode::ExactOut => "ExactOut",
                    }
                    .into(),
                ),
            ])
            .send()
            .await
            .context("quote request to jupiter")?;
        let quote: QueryResult = util::http_error_handling(response).await.with_context(|| {
            format!("error requesting jupiter route between {input_mint} and {output_mint}")
        })?;

        let route = quote.data.first().ok_or_else(|| {
            anyhow::anyhow!(
                "no route for swap. found {} routes, but none were usable",
                quote.data.len()
            )
        })?;

        Ok(route.clone())
    }

    /// Find the instructions and account lookup tables for a jupiter swap through mango
    ///
    /// It would be nice if we didn't have to pass input_mint/output_mint - the data is
    /// definitely in QueryRoute - but it's unclear how.
    pub async fn prepare_swap_transaction(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        route: &QueryRoute,
    ) -> anyhow::Result<TransactionBuilder> {
        let source_token = self.mango_client.context.token_by_mint(&input_mint)?;
        let target_token = self.mango_client.context.token_by_mint(&output_mint)?;

        let swap_response = self
            .mango_client
            .http_client
            .post("https://quote-api.jup.ag/v4/swap")
            .json(&SwapRequest {
                route: route.clone(),
                user_public_key: self.mango_client.owner.pubkey().to_string(),
                wrap_unwrap_sol: false,
                compute_unit_price_micro_lamports: None, // we already prioritize
            })
            .send()
            .await
            .context("swap transaction request to jupiter")?;

        let swap: SwapResponse = util::http_error_handling(swap_response)
            .await
            .context("error requesting jupiter swap")?;

        if swap.setup_transaction.is_some() || swap.cleanup_transaction.is_some() {
            anyhow::bail!(
                "chosen jupiter route requires setup or cleanup transactions, can't execute"
            );
        }

        let jup_tx = bincode::options()
            .with_fixint_encoding()
            .reject_trailing_bytes()
            .deserialize::<solana_sdk::transaction::VersionedTransaction>(
                &base64::decode(&swap.swap_transaction)
                    .context("base64 decoding jupiter transaction")?,
            )
            .context("parsing jupiter transaction")?;
        let ata_program = anchor_spl::associated_token::ID;
        let token_program = anchor_spl::token::ID;
        let compute_budget_program = solana_sdk::compute_budget::ID;
        // these setup instructions should be placed outside of flashloan begin-end
        let is_setup_ix = |k: Pubkey| -> bool {
            k == ata_program || k == token_program || k == compute_budget_program
        };
        let (jup_ixs, jup_alts) = self
            .mango_client
            .deserialize_instructions_and_alts(&jup_tx.message)
            .await?;
        let jup_action_ix_begin = jup_ixs
            .iter()
            .position(|ix| !is_setup_ix(ix.program_id))
            .ok_or_else(|| {
                anyhow::anyhow!("jupiter swap response only had setup-like instructions")
            })?;
        let jup_action_ix_end = jup_ixs.len()
            - jup_ixs
                .iter()
                .rev()
                .position(|ix| !is_setup_ix(ix.program_id))
                .unwrap();

        let bank_ams = [
            source_token.mint_info.first_bank(),
            target_token.mint_info.first_bank(),
        ]
        .into_iter()
        .map(util::to_writable_account_meta)
        .collect::<Vec<_>>();

        let vault_ams = [
            source_token.mint_info.first_vault(),
            target_token.mint_info.first_vault(),
        ]
        .into_iter()
        .map(util::to_writable_account_meta)
        .collect::<Vec<_>>();

        let owner = self.mango_client.owner();

        let token_ams = [source_token.mint_info.mint, target_token.mint_info.mint]
            .into_iter()
            .map(|mint| {
                util::to_writable_account_meta(
                    anchor_spl::associated_token::get_associated_token_address(&owner, &mint),
                )
            })
            .collect::<Vec<_>>();

        let source_loan = if route.swap_mode == "ExactIn" {
            u64::from_str(&route.amount).unwrap()
        } else if route.swap_mode == "ExactOut" {
            u64::from_str(&route.other_amount_threshold).unwrap()
        } else {
            anyhow::bail!("unknown swap mode: {}", route.swap_mode);
        };
        let loan_amounts = vec![source_loan, 0u64];
        let num_loans: u8 = loan_amounts.len().try_into().unwrap();

        // This relies on the fact that health account banks will be identical to the first_bank above!
        let health_ams = self
            .mango_client
            .derive_health_check_remaining_account_metas(
                vec![source_token.token_index, target_token.token_index],
                vec![source_token.token_index, target_token.token_index],
                vec![],
            )
            .await
            .context("building health accounts")?;

        let mut instructions = Vec::new();

        for ix in &jup_ixs[..jup_action_ix_begin] {
            instructions.push(ix.clone());
        }

        // Ensure the source token account is created (jupiter takes care of the output account)
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &owner,
                &owner,
                &source_token.mint_info.mint,
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
        for ix in &jup_ixs[jup_action_ix_begin..jup_action_ix_end] {
            instructions.push(ix.clone());
        }
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
        for ix in &jup_ixs[jup_action_ix_end..] {
            instructions.push(ix.clone());
        }

        let mut address_lookup_tables = self.mango_client.mango_address_lookup_tables().await?;
        address_lookup_tables.extend(jup_alts.into_iter());

        let payer = owner; // maybe use fee_payer? but usually it's the same

        Ok(TransactionBuilder {
            instructions,
            address_lookup_tables,
            payer,
            signers: vec![self.mango_client.owner.clone()],
            config: self.mango_client.client.transaction_builder_config,
        })
    }

    pub async fn swap(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
        slippage_bps: u64,
        swap_mode: JupiterSwapMode,
        only_direct_routes: bool,
    ) -> anyhow::Result<Signature> {
        let route = self
            .quote(
                input_mint,
                output_mint,
                amount,
                slippage_bps,
                swap_mode,
                only_direct_routes,
            )
            .await?;

        let tx_builder = self
            .prepare_swap_transaction(input_mint, output_mint, &route)
            .await?;

        tx_builder.send_and_confirm(&self.mango_client.client).await
    }
}
