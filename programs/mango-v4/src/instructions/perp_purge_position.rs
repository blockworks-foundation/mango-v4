use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::error::*;
use crate::logs::emit_perp_balances;
use crate::logs::emit_stack;
use crate::logs::TokenBalanceLog;
use crate::require_msg;
use crate::state::*;

pub fn perp_purge_position(ctx: Context<PerpPurgePosition>) -> Result<()> {
    let perp_market = ctx.accounts.perp_market.load()?;
    // only allow purging positions when market is in force-close
    require!(perp_market.is_force_close(), MangoError::SomeError);

    let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;
    // Verify that the bank is the quote currency bank
    require!(
        settle_bank.token_index == perp_market.settle_token_index,
        MangoError::InvalidBank
    );

    let mut account = ctx.accounts.account.load_full_mut()?;
    let perp_position = account.perp_position_mut(perp_market.perp_market_index)?;

    // Base position needs to be zero'd out already and all orders closed
    require_msg!(
        perp_position.base_position_lots() == 0,
        "perp position still has base lots"
    );
    require_msg!(
        perp_position.bids_base_lots == 0 && perp_position.asks_base_lots == 0,
        "perp position still has open orders"
    );
    require_msg!(
        perp_position.taker_base_lots == 0 && perp_position.taker_quote_lots == 0,
        "perp position still has events on event queue"
    );

    // Flush funding so that all remaining quote position is accounted for
    perp_position.settle_funding(&perp_market);
    let settlement = -perp_position.quote_position_native();

    // only if there's negative pnl we settle directly with bank
    // note: the positive pnl case is not implemented
    if settlement != 0 {
        require_msg!(
            settlement > I80F48::ZERO,
            "can only purge negative quote positions"
        );

        // Set perp quote to 0
        perp_position.record_settle(settlement, &perp_market);
        emit_perp_balances(
            ctx.accounts.group.key(),
            ctx.accounts.account.key(),
            perp_position,
            &perp_market,
        );

        // Update the accounts' perp_spot_transfer statistics.
        let settlement_i64 = settlement.round_to_zero().to_num::<i64>();
        perp_position.perp_spot_transfers += settlement_i64;
        account.fixed.perp_spot_transfers += settlement_i64;

        // Settle quote token balance
        let token_position = account
            .token_position_mut(perp_market.settle_token_index)?
            .0;
        let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        settle_bank.withdraw_without_fee(token_position, settlement, now_ts)?;

        emit_stack(TokenBalanceLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            token_index: perp_market.settle_token_index,
            indexed_position: token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });
    }

    // clean up perp position to free up oracles for users and allow closing market
    account.deactivate_perp_position_and_log(
        perp_market.perp_market_index,
        perp_market.settle_token_index,
        ctx.accounts.account.key(),
    )?;

    Ok(())
}
