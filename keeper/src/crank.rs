use std::sync::Arc;

use crate::{consume_events, update_funding, update_index, MangoClient};

use futures::Future;

pub async fn runner(
    mango_client: Arc<MangoClient>,
    debugging_handle: impl Future,
) -> Result<(), anyhow::Error> {
    let handles1 = mango_client
        .banks_cache
        .values()
        .map(|(pk, bank)| update_index::loop_blocking(mango_client.clone(), *pk, *bank))
        .collect::<Vec<_>>();

    let handles2 = mango_client
        .perp_markets_cache
        .values()
        .map(|(pk, perp_market)| {
            // todo: inline entire call
            consume_events::loop_blocking(mango_client.clone(), *pk, *perp_market)
        })
        .collect::<Vec<_>>();

    let handles3 = mango_client
        .perp_markets_cache
        .values()
        .map(|(pk, perp_market)| {
            // todo: inline entire call
            update_funding::loop_blocking(mango_client.clone(), *pk, *perp_market)
        })
        .collect::<Vec<_>>();

    futures::join!(
        futures::future::join_all(handles1),
        futures::future::join_all(handles2),
        futures::future::join_all(handles3),
        debugging_handle
    );

    Ok(())
}
