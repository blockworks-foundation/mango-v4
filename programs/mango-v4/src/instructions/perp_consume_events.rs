use anchor_lang::prelude::*;
use bytemuck::cast_ref;

use crate::accounts_zerocopy::*;
use crate::error::MangoError;
use crate::state::EventQueue;
use crate::state::{EventType, FillEvent, Group, MangoAccount, OutEvent, PerpMarket};

use crate::logs::{emit_perp_balances, mango_emit_stack, FillLog};

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
                    let mut ma = match mango_account_ais.iter().find(|ai| ai.key == &fill.maker) {
                        None => {
                            msg!("Unable to find account {}", fill.maker.to_string());
                            return Ok(());
                        }
                        Some(account_info) => account_info.load_mut::<MangoAccount>()?,
                    };

                    ma.perps.execute_maker(
                        perp_market.perp_market_index,
                        &mut perp_market,
                        fill,
                    )?;
                    ma.perps.execute_taker(
                        perp_market.perp_market_index,
                        &mut perp_market,
                        fill,
                    )?;
                    emit_perp_balances(
                        fill.maker,
                        perp_market.perp_market_index as u64,
                        fill.price,
                        &ma.perps.accounts[perp_market.perp_market_index as usize],
                        &perp_market,
                    );
                } else {
                    let mut maker = match mango_account_ais.iter().find(|ai| ai.key == &fill.maker)
                    {
                        None => {
                            msg!("Unable to find maker account {}", fill.maker.to_string());
                            return Ok(());
                        }
                        Some(account_info) => account_info.load_mut::<MangoAccount>()?,
                    };
                    let mut taker = match mango_account_ais.iter().find(|ai| ai.key == &fill.taker)
                    {
                        None => {
                            msg!("Unable to find taker account {}", fill.taker.to_string());
                            return Ok(());
                        }
                        Some(account_info) => account_info.load_mut::<MangoAccount>()?,
                    };

                    maker.perps.execute_maker(
                        perp_market.perp_market_index,
                        &mut perp_market,
                        fill,
                    )?;
                    taker.perps.execute_taker(
                        perp_market.perp_market_index,
                        &mut perp_market,
                        fill,
                    )?;
                    emit_perp_balances(
                        fill.maker,
                        perp_market.perp_market_index as u64,
                        fill.price,
                        &maker.perps.accounts[perp_market.perp_market_index as usize],
                        &perp_market,
                    );
                    emit_perp_balances(
                        fill.taker,
                        perp_market.perp_market_index as u64,
                        fill.price,
                        &taker.perps.accounts[perp_market.perp_market_index as usize],
                        &perp_market,
                    );
                }
                mango_emit_stack::<_, 512>(FillLog {
                    mango_group: ctx.accounts.group.key(),
                    market_index: perp_market.perp_market_index,
                    taker_side: fill.taker_side as u8,
                    maker_slot: fill.maker_slot,
                    market_fees_applied: fill.market_fees_applied,
                    maker_out: fill.maker_out,
                    timestamp: fill.timestamp,
                    seq_num: fill.seq_num,
                    maker: fill.maker,
                    maker_order_id: fill.maker_order_id,
                    maker_client_order_id: fill.maker_client_order_id,
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

                let mut ma = match mango_account_ais.iter().find(|ai| ai.key == &out.owner) {
                    None => {
                        msg!("Unable to find account {}", out.owner.to_string());
                        return Ok(());
                    }
                    Some(account_info) => account_info.load_mut::<MangoAccount>()?,
                };

                ma.perps
                    .remove_order(out.owner_slot as usize, out.quantity)?;
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
