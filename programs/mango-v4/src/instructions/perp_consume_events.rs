use anchor_lang::prelude::*;
use bytemuck::cast_ref;

use crate::error::MangoError;
use crate::state::{AccountLoaderDynamic, EventQueue, MangoAccount};
use crate::state::{EventType, FillEvent, Group, OutEvent, PerpMarket};

use crate::logs::{emit_perp_balances, FillLog};

#[derive(Accounts)]
pub struct PerpConsumeEvents<'info> {
    pub group: AccountLoader<'info, Group>,

    #[account(
        mut,
        has_one = group,
    )]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(mut)]
    pub event_queue: AccountLoader<'info, EventQueue>,
}

pub fn perp_consume_events(ctx: Context<PerpConsumeEvents>, limit: usize) -> Result<()> {
    let group = ctx.accounts.group.load()?;

    let limit = std::cmp::min(limit, 8);

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
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
                if fill.maker == fill.taker {
                    match mango_account_ais.iter().find(|ai| ai.key == &fill.maker) {
                        None => {
                            msg!("Unable to find account {}", fill.maker.to_string());
                            return Ok(());
                        }

                        Some(ai) => {
                            if group.is_testing() && ai.owner != &crate::id() {
                                msg!("Mango account (taker) not owned by mango program");
                                event_queue.pop_front()?;
                                continue;
                            }

                            let mal: AccountLoaderDynamic<MangoAccount> =
                                AccountLoaderDynamic::try_from(ai)?;
                            let mut ma = mal.load_mut()?;
                            ma.execute_perp_maker(
                                perp_market.perp_market_index,
                                &mut perp_market,
                                fill,
                            )?;
                            ma.execute_perp_taker(
                                perp_market.perp_market_index,
                                &mut perp_market,
                                fill,
                            )?;
                            emit_perp_balances(
                                ctx.accounts.group.key(),
                                fill.maker,
                                perp_market.perp_market_index,
                                ma.perp_position(perp_market.perp_market_index).unwrap(),
                                &perp_market,
                            );
                        }
                    };
                } else {
                    match mango_account_ais.iter().find(|ai| ai.key == &fill.maker) {
                        None => {
                            msg!("Unable to find maker account {}", fill.maker.to_string());
                            return Ok(());
                        }
                        Some(ai) => {
                            if group.is_testing() && ai.owner != &crate::id() {
                                msg!("Mango account (taker) not owned by mango program");
                                event_queue.pop_front()?;
                                continue;
                            }

                            let mal: AccountLoaderDynamic<MangoAccount> =
                                AccountLoaderDynamic::try_from(ai)?;
                            let mut maker = mal.load_mut()?;

                            match mango_account_ais.iter().find(|ai| ai.key == &fill.taker) {
                                None => {
                                    msg!("Unable to find taker account {}", fill.taker.to_string());
                                    return Ok(());
                                }
                                Some(ai) => {
                                    if group.is_testing() && ai.owner != &crate::id() {
                                        msg!("Mango account (taker) not owned by mango program");
                                        event_queue.pop_front()?;
                                        continue;
                                    }

                                    let mal: AccountLoaderDynamic<MangoAccount> =
                                        AccountLoaderDynamic::try_from(ai)?;
                                    let mut taker = mal.load_mut()?;

                                    maker.execute_perp_maker(
                                        perp_market.perp_market_index,
                                        &mut perp_market,
                                        fill,
                                    )?;
                                    taker.execute_perp_taker(
                                        perp_market.perp_market_index,
                                        &mut perp_market,
                                        fill,
                                    )?;
                                    emit_perp_balances(
                                        ctx.accounts.group.key(),
                                        fill.maker,
                                        perp_market.perp_market_index,
                                        maker.perp_position(perp_market.perp_market_index).unwrap(),
                                        &perp_market,
                                    );
                                    emit_perp_balances(
                                        ctx.accounts.group.key(),
                                        fill.taker,
                                        perp_market.perp_market_index,
                                        taker.perp_position(perp_market.perp_market_index).unwrap(),
                                        &perp_market,
                                    );
                                }
                            };
                        }
                    };
                }
                emit!(FillLog {
                    mango_group: ctx.accounts.group.key(),
                    market_index: perp_market.perp_market_index,
                    taker_side: fill.taker_side as u8,
                    maker_slot: fill.maker_slot,
                    maker_out: fill.maker_out(),
                    timestamp: fill.timestamp,
                    seq_num: fill.seq_num,
                    maker: fill.maker,
                    maker_order_id: fill.maker_order_id,
                    maker_fee: fill.maker_fee.to_bits(),
                    maker_timestamp: fill.maker_timestamp,
                    taker: fill.taker,
                    taker_order_id: fill.taker_order_id,
                    taker_client_order_id: fill.taker_client_order_id,
                    taker_fee: fill.taker_fee.to_bits(),
                    price: fill.price,
                    quantity: fill.quantity,
                });
            }
            EventType::Out => {
                let out: &OutEvent = cast_ref(event);

                match mango_account_ais.iter().find(|ai| ai.key == &out.owner) {
                    None => {
                        msg!("Unable to find account {}", out.owner.to_string());
                        return Ok(());
                    }
                    Some(ai) => {
                        if group.is_testing() && ai.owner != &crate::id() {
                            msg!("Mango account (taker) not owned by mango program");
                            event_queue.pop_front()?;
                            continue;
                        }

                        let mal: AccountLoaderDynamic<MangoAccount> =
                            AccountLoaderDynamic::try_from(ai)?;
                        let mut ma = mal.load_mut()?;

                        ma.remove_perp_order(out.owner_slot as usize, out.quantity)?;
                    }
                };
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
