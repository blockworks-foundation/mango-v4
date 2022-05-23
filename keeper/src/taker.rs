use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fixed_macro::types::I80F48;
use futures::Future;
use mango_v4::instructions::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};

use tokio::time;

use crate::{util::retry, MangoClient};

pub async fn runner(
    mango_client: Arc<MangoClient>,
    _debugging_handle: impl Future,
) -> Result<(), anyhow::Error> {
    let mut account = mango_client.get_account()?.1;
    // Ensure some decent balance
    for (_, bank) in mango_client.banks_cache.values() {
        if account
            .tokens
            .get_mut_or_create(bank.token_index)?
            .0
            .native(bank)
            > I80F48!(100)
        {
            continue;
        }
        retry(|| mango_client.deposit(bank.name(), 50))?;
    }

    for (_, serum3_market) in mango_client.serum3_markets_cache.values() {
        if account.serum3.find(serum3_market.market_index).is_none() {
            retry(|| mango_client.serum3_create_open_orders(serum3_market.name()))?;
        }
    }

    let mut price_arcs = HashMap::new();
    for market_name in mango_client.serum3_markets_cache.keys() {
        let price = mango_client
            .get_oracle_price(market_name.split('/').collect::<Vec<&str>>()[0])
            .unwrap();
        price_arcs.insert(market_name.to_owned(), Arc::new(RwLock::new(price.price)));
    }

    let handles1 = mango_client
        .serum3_markets_cache
        .keys()
        .map(|market_name| {
            loop_blocking_price_update(
                mango_client.clone(),
                market_name.to_owned(),
                price_arcs.get(market_name).unwrap().clone(),
            )
        })
        .collect::<Vec<_>>();

    let handles2 = mango_client
        .serum3_markets_cache
        .keys()
        .map(|market_name| {
            loop_blocking_orders(
                mango_client.clone(),
                market_name.to_owned(),
                price_arcs.get(market_name).unwrap().clone(),
            )
        })
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2)
    );

    Ok(())
}

pub async fn loop_blocking_price_update(
    mango_client: Arc<MangoClient>,
    market_name: String,
    price: Arc<RwLock<i64>>,
) {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;

        let client1 = mango_client.clone();
        let market_name1 = market_name.clone();
        let price = price.clone();
        tokio::task::spawn_blocking(move || {
            || -> anyhow::Result<()> {
                let token_name = market_name1.split('/').collect::<Vec<&str>>()[0];
                let fresh_price = client1.get_oracle_price(token_name).unwrap();
                log::info!("{} Updated price is {:?}", token_name, fresh_price.price);
                if let Ok(mut price) = price.try_write() {
                    *price = fresh_price.price;
                }
                Ok(())
            }()
            .expect("Something went wrong here...");
        })
        .await
        .expect("Something went wrong here...");
    }
}
pub async fn loop_blocking_orders(
    mango_client: Arc<MangoClient>,
    market_name: String,
    price: Arc<RwLock<i64>>,
) {
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let client = mango_client.clone();
        let market_name = market_name.clone();
        let price = price.clone();
        tokio::task::spawn_blocking(move || {
            || -> anyhow::Result<()> {
                let orders: Vec<u128> = client.serum3_cancel_all_orders(&market_name)?;
                log::info!("Cancelled orders - {:?} for {}", orders, market_name);

                let fresh_price = match price.read() {
                    Ok(price) => *price,
                    Err(_) => {
                        return Ok(());
                    }
                };

                let mut bid_price = fresh_price as f64 / 10u64.pow(8) as f64;
                bid_price = bid_price + bid_price * 0.01;
                client.serum3_place_order(
                    &market_name,
                    Serum3Side::Bid,
                    bid_price,
                    0.0001,
                    Serum3SelfTradeBehavior::DecrementTake,
                    Serum3OrderType::Limit,
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                    10,
                )?;

                let mut ask_price = fresh_price as f64 / 10u64.pow(8) as f64;
                ask_price = ask_price - ask_price * 0.01;
                client.serum3_place_order(
                    &market_name,
                    Serum3Side::Ask,
                    ask_price,
                    0.0001,
                    Serum3SelfTradeBehavior::DecrementTake,
                    Serum3OrderType::Limit,
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                    10,
                )?;

                log::info!(
                    "Placed bid at {}, and ask at {} for {}",
                    bid_price,
                    ask_price,
                    market_name
                );

                Ok(())
            }()
            .expect("Something went wrong here...");
        })
        .await
        .expect("Something went wrong here...");
    }
}
