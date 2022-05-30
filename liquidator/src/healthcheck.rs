use {
    crate::chain_data::ChainData,
    crate::websocket_sink::{HealthInfo, LiquidationCanditate},
    crate::Config,
    anyhow::Context,
    fixed::types::I80F48,
    log::*,
    mango::state::{
        DataType, HealthCache, HealthType, MangoAccount, MangoCache, MangoGroup, UserActiveAssets,
        MAX_PAIRS,
    },
    mango_common::Loadable,
    solana_sdk::account::{AccountSharedData, ReadableAccount},
    solana_sdk::pubkey::Pubkey,
    std::collections::HashSet,
    tokio::sync::broadcast,
};

// FUTURE: It'd be very nice if I could map T to the DataType::T constant!
pub fn load_mango_account<T: Loadable + Sized>(
    data_type: DataType,
    account: &AccountSharedData,
) -> anyhow::Result<&T> {
    let data = account.data();
    let data_type_int = data_type as u8;
    if data.len() != std::mem::size_of::<T>() {
        anyhow::bail!(
            "bad account size for {}: {} expected {}",
            data_type_int,
            data.len(),
            std::mem::size_of::<T>()
        );
    }
    if data[0] != data_type_int {
        anyhow::bail!(
            "unexpected data type for {}, got {}",
            data_type_int,
            data[0]
        );
    }
    return Ok(Loadable::load_from_bytes(&data).expect("always Ok"));
}

fn load_mango_account_from_chain<'a, T: Loadable + Sized>(
    data_type: DataType,
    chain_data: &'a ChainData,
    pubkey: &Pubkey,
) -> anyhow::Result<&'a T> {
    load_mango_account::<T>(
        data_type,
        chain_data
            .account(pubkey)
            .context("retrieving account from chain")?,
    )
}

pub fn load_open_orders_account(
    account: &AccountSharedData,
) -> anyhow::Result<&serum_dex::state::OpenOrders> {
    let data = account.data();
    let expected_size = 12 + std::mem::size_of::<serum_dex::state::OpenOrders>();
    if data.len() != expected_size {
        anyhow::bail!(
            "bad open orders account size: {} expected {}",
            data.len(),
            expected_size
        );
    }
    if &data[0..5] != "serum".as_bytes() {
        anyhow::bail!("unexpected open orders account prefix");
    }
    Ok(bytemuck::from_bytes::<serum_dex::state::OpenOrders>(
        &data[5..data.len() - 7],
    ))
}

fn get_open_orders<'a>(
    chain_data: &'a ChainData,
    group: &MangoGroup,
    account: &'a MangoAccount,
) -> anyhow::Result<Vec<Option<&'a serum_dex::state::OpenOrders>>> {
    let mut unpacked = vec![None; MAX_PAIRS];
    for i in 0..group.num_oracles {
        if account.in_margin_basket[i] {
            let oo = chain_data.account(&account.spot_open_orders[i])?;
            unpacked[i] = Some(load_open_orders_account(oo)?);
        }
    }
    Ok(unpacked)
}

#[derive(Debug)]
struct Health {
    candidate: bool,
    being_liquidated: bool,
    health_fraction: I80F48, // always maint
    assets: I80F48,          // always maint
    liabilities: I80F48,     // always maint
}

fn check_health(
    config: &Config,
    group: &MangoGroup,
    cache: &MangoCache,
    account: &MangoAccount,
    open_orders: &Vec<Option<&serum_dex::state::OpenOrders>>,
) -> anyhow::Result<Health> {
    let assets = UserActiveAssets::new(group, account, vec![]);
    let mut health_cache = HealthCache::new(assets);
    health_cache.init_vals_with_orders_vec(group, cache, account, open_orders)?;

    let (assets, liabilities) = health_cache.get_health_components(group, HealthType::Maint);
    let health_fraction = if liabilities > 0 {
        assets / liabilities
    } else {
        I80F48::MAX
    };

    let still_being_liquidated =
        account.being_liquidated && health_cache.get_health(group, HealthType::Init) < 0;

    let threshold = 1.0 + config.early_candidate_percentage / 100.0;
    let candidate = health_fraction < threshold || still_being_liquidated;

    Ok(Health {
        candidate,
        being_liquidated: still_being_liquidated,
        health_fraction,
        assets,
        liabilities,
    })
}

pub fn process_accounts<'a>(
    config: &Config,
    chain_data: &ChainData,
    group_id: &Pubkey,
    cache_id: &Pubkey,
    accounts: impl Iterator<Item = &'a Pubkey>,
    current_candidates: &mut HashSet<Pubkey>,
    tx: &broadcast::Sender<LiquidationCanditate>,
) -> anyhow::Result<()> {
    let group =
        load_mango_account_from_chain::<MangoGroup>(DataType::MangoGroup, chain_data, group_id)
            .context("loading group account")?;
    let cache =
        load_mango_account_from_chain::<MangoCache>(DataType::MangoCache, chain_data, cache_id)
            .context("loading cache account")?;

    for pubkey in accounts {
        let account_result = load_mango_account_from_chain::<MangoAccount>(
            DataType::MangoAccount,
            chain_data,
            pubkey,
        );
        let account = match account_result {
            Ok(account) => account,
            Err(err) => {
                warn!("could not load account {}: {:?}", pubkey, err);
                continue;
            }
        };
        let oos = match get_open_orders(chain_data, group, account) {
            Ok(oos) => oos,
            Err(err) => {
                warn!("could not load account {} open orders: {:?}", pubkey, err);
                continue;
            }
        };

        let info = match check_health(config, group, cache, account, &oos) {
            Ok(d) => d,
            Err(err) => {
                warn!("error computing health of {}: {:?}", pubkey, err);
                continue;
            }
        };

        let health_info = HealthInfo {
            account: pubkey.clone(),
            being_liquidated: info.being_liquidated,
            health_fraction: info.health_fraction,
            assets: info.assets,
            liabilities: info.liabilities,
        };

        let is_candidate = info.candidate;
        let was_candidate = current_candidates.contains(pubkey);
        if is_candidate && !was_candidate {
            info!("account {} is a new candidate", pubkey);
            current_candidates.insert(pubkey.clone());
            let _ = tx.send(LiquidationCanditate::Start {
                info: health_info.clone(),
            });
        }
        if is_candidate {
            let _ = tx.send(LiquidationCanditate::Now {
                info: health_info.clone(),
            });
        }
        if !is_candidate && was_candidate {
            info!("account {} stopped being a candidate", pubkey);
            current_candidates.remove(pubkey);
            let _ = tx.send(LiquidationCanditate::Stop {
                info: health_info.clone(),
            });
        }
    }

    Ok(())
}
