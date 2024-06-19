use anchor_lang::err;
use clap::Parser;
use fixed::types::I80F48;
use mango_v4::state::{Bank, TokenIndex, QUOTE_TOKEN_INDEX};
use mango_v4_client::swap::{Quote, Version};
use mango_v4_client::{
    keypair_from_cli, swap, Client, MangoClient, TokenContext, TransactionBuilderConfig,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[derive(Parser)]
#[clap()]
struct Cli {
    #[clap(short, long, env)]
    pub(crate) rpc_url: String,

    #[clap(long, env)]
    pub(crate) mango_account: String,

    #[clap(long, env)]
    pub(crate) owner: String,

    #[clap(long, env, default_value = "")]
    pub(crate) jupiter_token: String,
}

#[tokio::test]
#[ignore = "manual test"]
async fn should_test_all_swap() {
    // TODO FAS config
    let config = Cli::parse_from(dotenv::dotenv().ok());
    mango_v4_client::tracing_subscriber_init();

    let jup_token = config.jupiter_token;
    let client = client(config.rpc_url, jup_token.to_string()).expect("client");
    let owner = Arc::new(keypair_from_cli(config.owner.as_str()));
    let account = config.mango_account.as_str();

    let mango_client =
        MangoClient::new_for_existing_account(client, Pubkey::from_str(account).unwrap(), owner)
            .await
            .expect("mango client");
    let quote = mango_client.context.token(QUOTE_TOKEN_INDEX).mint;
    let sol = mango_client.context.token_by_name("SOL").mint;

    let rpc = mango_client.client.new_rpc_async();
    let lst = swap::sanctum::load_supported_token_mints(&rpc)
        .await
        .expect("couldnt load LST");

    for (token_index, token) in &mango_client.context.tokens {
        if token.mint == quote {
            continue;
        }

        let bank = mango_client.first_bank(*token_index).await.expect("bank");
        if bank.disable_asset_liquidation != 0 {
            debug!("Token: {} ignored (not liquidatable)", token.name);
            continue;
        }

        let price = mango_client
            .bank_oracle_price(*token_index)
            .await
            .expect("price");
        let buy_amount_in_usdc = 1_000_000 * 50_000;
        let sell_amount = (I80F48::from_num(buy_amount_in_usdc) / price).to_num();

        let can_buy = can_swap(&mango_client, quote, token.mint, buy_amount_in_usdc, &lst)
            .await
            .expect("can buy (sol) unwrap")
            || !can_swap(
                &mango_client,
                sol,
                token.mint,
                buy_amount_in_usdc * 1000 / 150,
                &lst,
            )
            .await
            .expect("can buy unwrap");

        let can_sell = can_swap(&mango_client, token.mint, quote, sell_amount, &lst)
            .await
            .expect("can sell unwrap")
            || can_swap(&mango_client, token.mint, sol, sell_amount, &lst)
                .await
                .expect("can sell (sol) unwrap");

        if can_buy && can_sell {
            info!(
                "Token: {} with buy_amount_in_usdc {} and sell_amount_in_token {} (price {}) -> Buy/Sell OK",
                token.name, buy_amount_in_usdc, sell_amount, price
            );
        } else {
            error!(
                "Token: {} with buy_amount_in_usdc {} and sell_amount_in_token {} (price {}) -> can_sell: {}, can_buy: {}",
                token.name, buy_amount_in_usdc, sell_amount, price, can_sell, can_buy
            );
        }
    }
}

async fn can_swap(
    mc: &MangoClient,
    input: Pubkey,
    output: Pubkey,
    amount: u64,
    lst: &HashSet<Pubkey>,
) -> anyhow::Result<bool> {
    let jup_all_routes = mc
        .swap()
        .quote(input, output, amount, 50, false, Version::V6)
        .await;
    if is_route_ok(mc, jup_all_routes).await? {
        return Ok(true);
    }

    let jup_direct_routes = mc
        .swap()
        .quote(input, output, amount, 50, true, Version::V6)
        .await;
    if is_route_ok(mc, jup_direct_routes).await? {
        return Ok(true);
    }

    if !lst.contains(&input) && !lst.contains(&output) {
        return Ok(false);
    }

    let sanctum_route = mc
        .swap()
        .quote(input, output, amount, 50, true, Version::Sanctum)
        .await;
    return Ok(is_route_ok(mc, sanctum_route).await?);
}

async fn is_route_ok(mc: &MangoClient, quote: anyhow::Result<Quote>) -> anyhow::Result<bool> {
    let quote = match quote {
        Ok(q) => q,
        Err(e) => {
            // println!("no quote found: {}", e);
            return Ok(false);
        }
    };

    let builder = mc.swap().prepare_swap_transaction(&quote).await?;
    let tx_size = builder.transaction_size()?;
    if !tx_size.is_within_limit() {
        error!("tx too big");
        return Ok(false);
    }

    Ok(quote.price_impact_pct < 50.0)
}

fn client(rpc_http_url: String, jup_token: String) -> anyhow::Result<Client> {
    let fee_payer = Keypair::new();
    Ok(Client::builder()
        .cluster(anchor_client::Cluster::from_str(&rpc_http_url).expect("failed to create cluster"))
        .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
        .fee_payer(Some(Arc::new(fee_payer)))
        .transaction_builder_config(
            TransactionBuilderConfig::builder()
                .prioritization_micro_lamports(Some(5))
                .compute_budget_per_instruction(Some(250_000))
                .build()
                .unwrap(),
        )
        .jupiter_token(jup_token)
        .build()
        .unwrap())
}
