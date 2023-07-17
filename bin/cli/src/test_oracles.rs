use itertools::Itertools;
use mango_v4::accounts_zerocopy::KeyedAccount;
use mango_v4_client::{Client, MangoGroupContext};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tracing::*;

pub async fn run(client: &Client, group: Pubkey) -> anyhow::Result<()> {
    let rpc_async = client.rpc_async();
    let context = MangoGroupContext::new_from_rpc(&rpc_async, group).await?;
    let oracles = context
        .tokens
        .values()
        .map(|t| t.mint_info.oracle)
        .chain(context.perp_markets.values().map(|p| p.market.oracle))
        .unique()
        .collect_vec();

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        interval.tick().await;

        let response = rpc_async
            .get_multiple_accounts_with_commitment(&oracles, CommitmentConfig::processed())
            .await;
        if response.is_err() {
            warn!("could not fetch oracles");
            continue;
        }
        let response = response.unwrap();
        let slot = response.context.slot;
        let accounts = response.value;

        for (pubkey, account_opt) in oracles.iter().zip(accounts.into_iter()) {
            if account_opt.is_none() {
                warn!("no oracle data for {pubkey}");
                continue;
            }
            let keyed_account = KeyedAccount {
                key: *pubkey,
                account: account_opt.unwrap(),
            };

            let tc_opt = context
                .tokens
                .values()
                .find(|t| t.mint_info.oracle == *pubkey);
            let pc_opt = context
                .perp_markets
                .values()
                .find(|p| p.market.oracle == *pubkey);
            let mut price = None;
            if let Some(tc) = tc_opt {
                match tc.bank.oracle_price(&keyed_account, Some(slot)) {
                    Ok(p) => price = Some(p),
                    Err(e) => {
                        error!("could not read bank oracle {}: {e:?}", keyed_account.key);
                    }
                }
            }
            if let Some(pc) = pc_opt {
                match pc.market.oracle_price(&keyed_account, Some(slot)) {
                    Ok(p) => price = Some(p),
                    Err(e) => {
                        error!("could not read perp oracle {}: {e:?}", keyed_account.key);
                    }
                }
            }
            if let Some(p) = price {
                info!("{pubkey},{p}");
            }
        }
    }
}
