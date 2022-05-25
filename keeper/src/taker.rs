use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fixed::types::I80F48;
use futures::Future;
use mango_v4::instructions::{Serum3OrderType, Serum3SelfTradeBehavior, Serum3Side};

use tokio::time;

use crate::MangoClient;

pub async fn runner(
    mango_client: Arc<MangoClient>,
    _debugging_handle: impl Future,
) -> Result<(), anyhow::Error> {
    ensure_deposit(&mango_client)?;

    ensure_oo(&mango_client)?;

    let mut price_arcs = HashMap::new();
    for market_name in mango_client.serum3_markets_cache.keys() {
        let price = mango_client
            .get_oracle_price(
                market_name
                    .split('/')
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap(),
            )
            .unwrap();
        price_arcs.insert(
            market_name.to_owned(),
            Arc::new(RwLock::new(
                I80F48::from_num(price.price) / I80F48::from_num(10u64.pow(-price.expo as u32)),
            )),
        );
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

fn ensure_oo(mango_client: &Arc<MangoClient>) -> Result<(), anyhow::Error> {
    let account = mango_client.get_account()?.1;

    for (_, serum3_market) in mango_client.serum3_markets_cache.values() {
        if account.serum3.find(serum3_market.market_index).is_none() {
            mango_client.serum3_create_open_orders(serum3_market.name())?;
        }
    }

    Ok(())
}

fn ensure_deposit(mango_client: &Arc<MangoClient>) -> Result<(), anyhow::Error> {
    let mango_account = mango_client.get_account()?.1;

    for (_, bank) in mango_client.banks_cache.values() {
        let mint = &mango_client.mint_infos_cache.get(&bank.mint).unwrap().2;
        let desired_balance = I80F48::from_num(10_000 * 10u64.pow(mint.decimals as u32));

        let token_account = mango_account.tokens.find(bank.token_index).unwrap();
        let native = token_account.native(bank);

        let ui = token_account.ui(bank, mint);
        log::info!("Current balance {} {}", ui, bank.name());

        let deposit_native = if native < I80F48::ZERO {
            desired_balance - native
        } else {
            desired_balance - native.min(desired_balance)
        };

        if deposit_native == I80F48::ZERO {
            continue;
        }

        log::info!("Depositing {} {}", deposit_native, bank.name());
        mango_client.deposit(bank.name(), desired_balance.to_num())?;
    }

    Ok(())
}

pub async fn loop_blocking_price_update(
    mango_client: Arc<MangoClient>,
    market_name: String,
    price: Arc<RwLock<I80F48>>,
) {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;

        let client1 = mango_client.clone();
        let market_name1 = market_name.clone();
        let price = price.clone();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let token_name = market_name1.split('/').collect::<Vec<&str>>()[0];
            let fresh_price = client1.get_oracle_price(token_name).unwrap();
            log::info!("{} Updated price is {:?}", token_name, fresh_price.price);
            if let Ok(mut price) = price.write() {
                *price = I80F48::from_num(fresh_price.price)
                    / I80F48::from_num(10u64.pow(-fresh_price.expo as u32));
            }
            Ok(())
        });
    }
}

pub async fn loop_blocking_orders(
    mango_client: Arc<MangoClient>,
    market_name: String,
    price: Arc<RwLock<I80F48>>,
) {
    let mut interval = time::interval(Duration::from_secs(5));

    // Cancel existing orders
    let orders: Vec<u128> = mango_client.serum3_cancel_all_orders(&market_name).unwrap();
    log::info!("Cancelled orders - {:?} for {}", orders, market_name);

    loop {
        interval.tick().await;

        let client = mango_client.clone();
        let market_name = market_name.clone();
        let price = price.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            client.serum3_settle_funds(&market_name)?;

            let fresh_price = match price.read() {
                Ok(price) => *price,
                Err(err) => {
                    anyhow::bail!("Price RwLock PoisonError!");
                }
            };

            let fresh_price = fresh_price.to_num::<f64>();

            let bid_price = fresh_price + fresh_price * 0.1;
            let res = client.serum3_place_order(
                &market_name,
                Serum3Side::Bid,
                bid_price,
                0.0001,
                Serum3SelfTradeBehavior::DecrementTake,
                Serum3OrderType::ImmediateOrCancel,
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                10,
            );
            if let Err(e) = res {
                log::error!("Error while placing taker bid {:#?}", e)
            } else {
                log::info!("Placed bid at {} for {}", bid_price, market_name)
            }

            let ask_price = fresh_price - fresh_price * 0.1;
            let res = client.serum3_place_order(
                &market_name,
                Serum3Side::Ask,
                ask_price,
                0.0001,
                Serum3SelfTradeBehavior::DecrementTake,
                Serum3OrderType::ImmediateOrCancel,
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
                10,
            );
            if let Err(e) = res {
                log::error!("Error while placing taker ask {:#?}", e)
            } else {
                log::info!("Placed ask at {} for {}", ask_price, market_name)
            }

            Ok(())
        });
    }
}
