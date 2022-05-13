use std::sync::Arc;

use crate::{consume_events, update_index, MangoClient};

use anyhow::ensure;

use futures::Future;

use mango_v4::state::{Bank, PerpMarket};

use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};

use solana_sdk::{pubkey::Pubkey, signer::Signer};

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
) -> Result<(), anyhow::Error> {
    // Collect all banks for a group belonging to an admin
    let banks = mango_client
        .program()
        .accounts::<Bank>(vec![RpcFilterType::Memcmp(Memcmp {
            offset: 24,
            bytes: MemcmpEncodedBytes::Base58({
                // find group belonging to admin
                Pubkey::find_program_address(
                    &["Group".as_ref(), mango_client.admin.pubkey().as_ref()],
                    &mango_client.program().id(),
                )
                .0
                .to_string()
            }),
            encoding: None,
        })])?;

    ensure!(!banks.is_empty());

    let handles1 = banks
        .iter()
        .map(|(pk, bank)| update_index::loop_blocking(mango_client.clone(), *pk, *bank))
        .collect::<Vec<_>>();

    // TODO: future, maybe we want to only consume events for specific markets,
    // TODO: future, maybe we want to crank certain markets more often than others
    // Collect all perp markets for a group belonging to an admin
    let perp_markets =
        mango_client
            .program()
            .accounts::<PerpMarket>(vec![RpcFilterType::Memcmp(Memcmp {
                offset: 24,
                bytes: MemcmpEncodedBytes::Base58({
                    // find group belonging to admin
                    Pubkey::find_program_address(
                        &["Group".as_ref(), mango_client.admin.pubkey().as_ref()],
                        &mango_client.program().id(),
                    )
                    .0
                    .to_string()
                }),
                encoding: None,
            })])?;

    // TODO: enable
    // ensure!(!perp_markets.is_empty());
    // atm no perp code is deployed to devnet, and no perp markets have been init

    let handles2 = perp_markets
        .iter()
        .map(|(pk, perp_market)| {
            consume_events::loop_blocking(mango_client.clone(), *pk, *perp_market)
        })
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        debugging_handle
    );

    Ok(())
}
