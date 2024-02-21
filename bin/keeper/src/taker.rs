use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fixed::types::I80F48;
use futures::Future;
use mango_v4::{
    accounts_ix::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side},
    state::TokenIndex,
};
use tokio::task::JoinHandle;
use tracing::*;

use crate::MangoClient;

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
    extra_jobs: Vec<JoinHandle<()>>,
) -> Result<(), anyhow::Error> {
    ensure_deposit(&mango_client).await?;
    ensure_oo(&mango_client).await?;

    let mut price_arcs = HashMap::new();
    for s3_market in mango_client.context.serum3_markets.values() {
        let base_token_index = s3_market.base_token_index;
        let price = mango_client
            .bank_oracle_price(base_token_index)
            .await
            .unwrap();
        price_arcs.insert(base_token_index, Arc::new(RwLock::new(price)));
    }

    let handles1 = price_arcs
        .iter()
        .map(|(base_token_index, price)| {
            loop_blocking_price_update(mango_client.clone(), *base_token_index, price.clone())
        })
        .collect::<Vec<_>>();

    let handles2 = mango_client
        .context
        .serum3_markets
        .values()
        .map(|s3_market| {
            loop_blocking_orders(
                mango_client.clone(),
                s3_market.name.clone(),
                price_arcs.get(&s3_market.base_token_index).unwrap().clone(),
            )
        })
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        debugging_handle,
        futures::future::join_all(extra_jobs),
    );

    Ok(())
}

async fn ensure_oo(mango_client: &Arc<MangoClient>) -> Result<(), anyhow::Error> {
    let account = mango_client.mango_account().await?;

    for (market_index, serum3_market) in mango_client.context.serum3_markets.iter() {
        if account.serum3_orders(*market_index).is_err() {
            mango_client
                .serum3_create_open_orders(&serum3_market.name)
                .await?;
        }
    }

    Ok(())
}

async fn ensure_deposit(mango_client: &Arc<MangoClient>) -> Result<(), anyhow::Error> {
    let mango_account = mango_client.mango_account().await?;

    for &token_index in mango_client.context.tokens.keys() {
        let bank = mango_client.first_bank(token_index).await?;
        let desired_balance = I80F48::from_num(10_000 * 10u64.pow(bank.mint_decimals as u32));

        let token_account_opt = mango_account.token_position(token_index).ok();

        let deposit_native = match token_account_opt {
            Some(token_account) => {
                let native = token_account.native(&bank);
                let ui = token_account.ui(&bank);
                info!("Current balance {} {}", ui, bank.name());

                if native < I80F48::ZERO {
                    desired_balance - native
                } else {
                    desired_balance - native.min(desired_balance)
                }
            }
            None => desired_balance,
        };

        if deposit_native == I80F48::ZERO {
            continue;
        }

        info!("Depositing {} {}", deposit_native, bank.name());
        mango_client
            .token_deposit(bank.mint, desired_balance.to_num(), false)
            .await?;
    }

    Ok(())
}

pub async fn loop_blocking_price_update(
    mango_client: Arc<MangoClient>,
    token_index: TokenIndex,
    price: Arc<RwLock<I80F48>>,
) {
    let mut interval = mango_v4_client::delay_interval(Duration::from_secs(1));
    let token_name = &mango_client.context.token(token_index).name;
    loop {
        interval.tick().await;

        let fresh_price = mango_client.bank_oracle_price(token_index).await.unwrap();
        info!("{} Updated price is {:?}", token_name, fresh_price);
        if let Ok(mut price) = price.write() {
            *price = fresh_price;
        }
    }
}

pub async fn loop_blocking_orders(
    mango_client: Arc<MangoClient>,
    market_name: String,
    price: Arc<RwLock<I80F48>>,
) {
    let mut interval = mango_v4_client::delay_interval(Duration::from_secs(5));

    // Cancel existing orders
    let orders: Vec<u128> = mango_client
        .serum3_cancel_all_orders(&market_name)
        .await
        .unwrap();
    info!("Cancelled orders - {:?} for {}", orders, market_name);

    let market_index = mango_client.context.serum3_market_index(&market_name);
    let s3 = mango_client.context.serum3(market_index);

    loop {
        interval.tick().await;

        let client = mango_client.clone();
        let market_name = market_name.clone();
        let price = price.clone();

        let res: anyhow::Result<()> = (|| async move {
            client.serum3_settle_funds(&market_name).await?;

            let fresh_price = price.read().unwrap().to_num::<f64>();
            let bid_price = fresh_price * 1.1;

            let bid_price_lots = bid_price * s3.coin_lot_size as f64 / s3.pc_lot_size as f64;

            let res = client
                .serum3_place_order(
                    &market_name,
                    Serum3Side::Bid,
                    bid_price_lots.round() as u64,
                    1,
                    u64::MAX,
                    Serum3SelfTradeBehavior::DecrementTake,
                    Serum3OrderType::ImmediateOrCancel,
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                    10,
                )
                .await;
            if let Err(e) = res {
                error!("Error while placing taker bid {:#?}", e)
            } else {
                info!("Placed bid at {} for {}", bid_price, market_name)
            }

            let ask_price = fresh_price * 0.9;
            let ask_price_lots = ask_price * s3.coin_lot_size as f64 / s3.pc_lot_size as f64;

            let res = client
                .serum3_place_order(
                    &market_name,
                    Serum3Side::Ask,
                    ask_price_lots.round() as u64,
                    1,
                    u64::MAX,
                    Serum3SelfTradeBehavior::DecrementTake,
                    Serum3OrderType::ImmediateOrCancel,
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                    10,
                )
                .await;
            if let Err(e) = res {
                error!("Error while placing taker ask {:#?}", e)
            } else {
                info!("Placed ask at {} for {}", ask_price, market_name)
            }

            Ok(())
        })()
        .await;

        if let Err(err) = res {
            error!("{:?}", err);
        }
    }
}
