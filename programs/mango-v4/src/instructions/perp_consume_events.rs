use anchor_lang::prelude::*;
use bytemuck::cast_ref;

use crate::error::MangoError;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{emit_perp_balances, emit_stack, FillLogV3};

/// Load a mango account by key from the list of account infos.
///
/// Message and return Ok() if it's missing, to lock in successful processing
/// of previous events.
///
/// Special handling for testing groups, where events for accounts with bad
/// owners (most likely due to force closure of the account) are being skipped.
macro_rules! load_mango_account {
    ($name:ident, $key:expr, $ais:expr, $group:expr, $event_queue:expr) => {
        let loader = match $ais.iter().find(|ai| ai.key == &$key) {
            None => {
                msg!(
                    "Unable to find {} account {}",
                    stringify!($name),
                    $key.to_string()
                );
                return Ok(());
            }

            Some(ai) => {
                if $group.is_testing() && ai.owner != &crate::id() {
                    msg!(
                        "Mango account ({}) not owned by mango program",
                        stringify!($name)
                    );
                    $event_queue.pop_front()?;
                    continue;
                }

                let mal: AccountLoader<MangoAccountFixed> = AccountLoader::try_from(ai)?;
                mal
            }
        };
        let mut $name = loader.load_full_mut()?;
    };
}

pub fn perp_consume_events(ctx: Context<PerpConsumeEvents>, limit: usize) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_key = ctx.accounts.group.key();

    let limit = std::cmp::min(limit, 8);

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let perp_market_index = perp_market.perp_market_index;
    let mut event_queue = ctx.accounts.event_queue.load_mut()?;
    let mango_account_ais = &ctx.remaining_accounts;

    for _ in 0..limit {
        let event = match event_queue.peek_front() {
            None => break,
            Some(e) => e,
        };

        match EventType::try_from(event.event_type).map_err(|_| error!(MangoError::SomeError))? {
            EventType::Fill => {
                let fill: &FillEvent = cast_ref(event);

                // handle self trade separately because of rust borrow checker
                let (maker_closed_pnl, taker_closed_pnl) = if fill.maker == fill.taker {
                    load_mango_account!(
                        maker_taker,
                        fill.maker,
                        mango_account_ais,
                        group,
                        event_queue
                    );
                    let maker_realized_pnl = maker_taker.execute_perp_maker(
                        perp_market_index,
                        &mut perp_market,
                        fill,
                        &group,
                    )?;
                    let taker_realized_pnl = maker_taker.execute_perp_taker(
                        perp_market_index,
                        &mut perp_market,
                        fill,
                    )?;
                    emit_perp_balances(
                        group_key,
                        fill.maker,
                        maker_taker.perp_position(perp_market_index).unwrap(),
                        &perp_market,
                    );
                    let closed_pnl = maker_realized_pnl + taker_realized_pnl;
                    (closed_pnl, closed_pnl)
                } else {
                    load_mango_account!(maker, fill.maker, mango_account_ais, group, event_queue);
                    load_mango_account!(taker, fill.taker, mango_account_ais, group, event_queue);

                    let maker_realized_pnl = maker.execute_perp_maker(
                        perp_market_index,
                        &mut perp_market,
                        fill,
                        &group,
                    )?;
                    let taker_realized_pnl =
                        taker.execute_perp_taker(perp_market_index, &mut perp_market, fill)?;
                    emit_perp_balances(
                        group_key,
                        fill.maker,
                        maker.perp_position(perp_market_index).unwrap(),
                        &perp_market,
                    );
                    emit_perp_balances(
                        group_key,
                        fill.taker,
                        taker.perp_position(perp_market_index).unwrap(),
                        &perp_market,
                    );

                    (maker_realized_pnl, taker_realized_pnl)
                };
                emit_stack(FillLogV3 {
                    mango_group: group_key,
                    market_index: perp_market_index,
                    taker_side: fill.taker_side as u8,
                    maker_slot: fill.maker_slot,
                    maker_out: fill.maker_out(),
                    timestamp: fill.timestamp,
                    seq_num: fill.seq_num,
                    maker: fill.maker,
                    maker_client_order_id: fill.maker_client_order_id,
                    maker_fee: fill.maker_fee,
                    maker_timestamp: fill.maker_timestamp,
                    taker: fill.taker,
                    taker_client_order_id: fill.taker_client_order_id,
                    taker_fee: fill.taker_fee,
                    price: fill.price,
                    quantity: fill.quantity,
                    maker_closed_pnl: maker_closed_pnl.to_num(),
                    taker_closed_pnl: taker_closed_pnl.to_num(),
                });
            }
            EventType::Out => {
                let out: &OutEvent = cast_ref(event);
                load_mango_account!(owner, out.owner, mango_account_ais, group, event_queue);
                owner.execute_perp_out_event(
                    perp_market_index,
                    out.side(),
                    out.owner_slot as usize,
                    out.quantity,
                    out.order_id,
                )?;
            }
            EventType::Liquidate => {
                // This is purely for record keeping. Can be removed if program logs are superior
            }
        }

        // consume this event
        event_queue.pop_front()?;
    }
    Ok(())
}
