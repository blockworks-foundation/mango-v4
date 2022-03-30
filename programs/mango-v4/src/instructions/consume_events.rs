use anchor_lang::prelude::*;
use bytemuck::cast_ref;


use crate::error::MangoError;
use crate::{state::{PerpMarket, Group, Queue, EventType, FillEvent, OutEvent, EventQueueHeader, MangoAccount}, util::LoadZeroCopy};

#[derive(Accounts)]
pub struct ConsumeEvents<'info> {
    pub group: AccountLoader<'info, Group>,
    
    #[account(
        mut,
        has_one = group,    
    )]    
    pub perp_market: AccountLoader<'info, PerpMarket>,
    
    #[account(mut)]    
    pub event_queue: AccountLoader<'info, Queue<EventQueueHeader>>,
}

pub fn consume_events(ctx:Context<ConsumeEvents>, limit: usize) -> Result<()> {
    let limit = std::cmp::min(limit, 8);

    let mut perp_market =ctx.accounts.perp_market.load_mut()?;
    let mut event_queue = ctx.accounts.event_queue.load_mut()?;
    let mango_account_ais = &ctx.remaining_accounts;        

    for _ in 0..limit {
        let event = match event_queue.peek_front() {
            None => break,
            Some(e) => e,
        };

        match EventType::try_from(event.event_type).map_err(|_|error!(MangoError::SomeError))? {
            EventType::Fill => {
                let fill: &FillEvent = cast_ref(event);

                // handle self trade separately because of rust borrow checker
                if fill.maker == fill.taker {
                    let mut ma = match mango_account_ais.iter().find(|ai| ai.key == &fill.maker) {
                        None => {
                            msg!("Unable to find account {}", fill.maker.to_string());
                            return Ok(());
                        }
                        Some(account_info) =>account_info.load_mut::<MangoAccount>()?,
                    };
                    
                    ma.execute_maker(perp_market.perp_market_index, &mut perp_market, fill)?;
                    ma.execute_taker(perp_market.perp_market_index, &mut perp_market, fill)?;                                  
                } else {
                    let mut maker = match mango_account_ais.iter().find(|ai| ai.key == &fill.maker)
                    {
                        None => {
                            msg!("Unable to find maker account {}", fill.maker.to_string());
                            return Ok(());
                        }
                        Some(account_info) =>account_info.load_mut::<MangoAccount>()?,
                    };
                    let mut taker = match mango_account_ais.iter().find(|ai| ai.key == &fill.taker)
                    {
                        None => {
                            msg!("Unable to find taker account {}", fill.taker.to_string());
                            return Ok(());
                        }
                        Some(account_info) =>account_info.load_mut::<MangoAccount>()?,
                    };

                    maker.execute_maker(perp_market.perp_market_index, &mut perp_market, fill)?;
                    taker.execute_taker(perp_market.perp_market_index, &mut perp_market, fill)?;               
                }            
            }
            EventType::Out => {
                let out: &OutEvent = cast_ref(event);

                let mut ma = match mango_account_ais.iter().find(|ai| ai.key == &out.owner) {
                    None => {
                        msg!("Unable to find account {}", out.owner.to_string());
                        return Ok(());
                    }
                    Some(account_info) =>account_info.load_mut::<MangoAccount>()?,
                };

                ma.remove_order(out.owner_slot as usize, out.quantity)?;
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
