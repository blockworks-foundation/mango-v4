use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::{error::MangoError, state::*};

pub fn ix_gate_set(ctx: Context<IxGateSet>, ix_gate: u128) -> Result<()> {
    let mut group = ctx.accounts.group.load_mut()?;

    msg!("old  {:?}, new {:?}", group.ix_gate, ix_gate);

    let mut require_group_admin = false;
    for i in 0..128 {
        // only admin can re-enable
        if group.ix_gate & (1 << i) != 0 && ix_gate & (1 << i) == 0 {
            require_group_admin = true;
        }
    }

    log_if_changed(&group, ix_gate, IxGate::AccountClose);
    log_if_changed(&group, ix_gate, IxGate::AccountCreate);
    log_if_changed(&group, ix_gate, IxGate::AccountEdit);
    log_if_changed(&group, ix_gate, IxGate::AccountExpand);
    log_if_changed(&group, ix_gate, IxGate::AccountToggleFreeze);
    log_if_changed(&group, ix_gate, IxGate::AltExtend);
    log_if_changed(&group, ix_gate, IxGate::AltSet);
    log_if_changed(&group, ix_gate, IxGate::FlashLoan);
    log_if_changed(&group, ix_gate, IxGate::GroupClose);
    log_if_changed(&group, ix_gate, IxGate::GroupCreate);
    log_if_changed(&group, ix_gate, IxGate::HealthRegion);
    log_if_changed(&group, ix_gate, IxGate::PerpCancelAllOrders);
    log_if_changed(&group, ix_gate, IxGate::PerpCancelAllOrdersBySide);
    log_if_changed(&group, ix_gate, IxGate::PerpCancelOrder);
    log_if_changed(&group, ix_gate, IxGate::PerpCancelOrderByClientOrderId);
    log_if_changed(&group, ix_gate, IxGate::PerpCloseMarket);
    log_if_changed(&group, ix_gate, IxGate::PerpConsumeEvents);
    log_if_changed(&group, ix_gate, IxGate::PerpCreateMarket);
    log_if_changed(&group, ix_gate, IxGate::PerpDeactivatePosition);
    log_if_changed(&group, ix_gate, IxGate::PerpLiqBaseOrPositivePnl);
    log_if_changed(&group, ix_gate, IxGate::PerpLiqForceCancelOrders);
    log_if_changed(&group, ix_gate, IxGate::PerpLiqNegativePnlOrBankruptcy);
    log_if_changed(&group, ix_gate, IxGate::PerpPlaceOrder);
    log_if_changed(&group, ix_gate, IxGate::PerpSettleFees);
    log_if_changed(&group, ix_gate, IxGate::PerpSettlePnl);
    log_if_changed(&group, ix_gate, IxGate::PerpUpdateFunding);
    log_if_changed(&group, ix_gate, IxGate::Serum3CancelAllOrders);
    log_if_changed(&group, ix_gate, IxGate::Serum3CancelOrder);
    log_if_changed(&group, ix_gate, IxGate::Serum3CloseOpenOrders);
    log_if_changed(&group, ix_gate, IxGate::Serum3CreateOpenOrders);
    log_if_changed(&group, ix_gate, IxGate::Serum3DeregisterMarket);
    log_if_changed(&group, ix_gate, IxGate::Serum3LiqForceCancelOrders);
    log_if_changed(&group, ix_gate, IxGate::Serum3PlaceOrder);
    log_if_changed(&group, ix_gate, IxGate::Serum3RegisterMarket);
    log_if_changed(&group, ix_gate, IxGate::Serum3SettleFunds);
    log_if_changed(&group, ix_gate, IxGate::StubOracleClose);
    log_if_changed(&group, ix_gate, IxGate::StubOracleCreate);
    log_if_changed(&group, ix_gate, IxGate::StubOracleSet);
    log_if_changed(&group, ix_gate, IxGate::TokenAddBank);
    log_if_changed(&group, ix_gate, IxGate::TokenDeposit);
    log_if_changed(&group, ix_gate, IxGate::TokenDeregister);
    log_if_changed(&group, ix_gate, IxGate::TokenLiqBankruptcy);
    log_if_changed(&group, ix_gate, IxGate::TokenLiqWithToken);
    log_if_changed(&group, ix_gate, IxGate::TokenRegister);
    log_if_changed(&group, ix_gate, IxGate::TokenRegisterTrustless);
    log_if_changed(&group, ix_gate, IxGate::TokenUpdateIndexAndRate);
    log_if_changed(&group, ix_gate, IxGate::TokenWithdraw);
    log_if_changed(&group, ix_gate, IxGate::AccountBuybackFeesWithMngo);
    log_if_changed(&group, ix_gate, IxGate::TokenForceCloseBorrowsWithToken);
    log_if_changed(&group, ix_gate, IxGate::PerpForceClosePosition);
    log_if_changed(&group, ix_gate, IxGate::GroupWithdrawInsuranceFund);
    log_if_changed(&group, ix_gate, IxGate::TokenConditionalSwapCreate);
    log_if_changed(&group, ix_gate, IxGate::TokenConditionalSwapTrigger);
    log_if_changed(&group, ix_gate, IxGate::TokenConditionalSwapCancel);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2CancelOrder);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2CloseOpenOrders);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2CreateOpenOrders);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2DeregisterMarket);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2EditMarket);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2LiqForceCancelOrders);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2PlaceOrder);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2PlaceTakeOrder);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2RegisterMarket);
    log_if_changed(&group, ix_gate, IxGate::OpenbookV2SettleFunds);
    log_if_changed(&group, ix_gate, IxGate::AdminTokenWithdrawFees);
    log_if_changed(&group, ix_gate, IxGate::AdminPerpWithdrawFees);
    log_if_changed(&group, ix_gate, IxGate::AccountSizeMigration);
    log_if_changed(&group, ix_gate, IxGate::TokenConditionalSwapStart);
    log_if_changed(
        &group,
        ix_gate,
        IxGate::TokenConditionalSwapCreatePremiumAuction,
    );
    log_if_changed(
        &group,
        ix_gate,
        IxGate::TokenConditionalSwapCreateLinearAuction,
    );
    log_if_changed(&group, ix_gate, IxGate::Serum3PlaceOrderV2);
    log_if_changed(&group, ix_gate, IxGate::TokenForceWithdraw);

    group.ix_gate = ix_gate;

    // account constraint #1
    if require_group_admin {
        require!(
            group.admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    } else {
        require!(
            group.admin == ctx.accounts.admin.key()
                || group.security_admin == ctx.accounts.admin.key(),
            MangoError::SomeError
        );
    }
    Ok(())
}

fn log_if_changed(group: &Group, ix_gate: u128, ix: IxGate) {
    let old = group.is_ix_enabled(ix);
    let new = ix_gate & (1 << ix as u128) == 0;
    if old != new {
        msg!(
            "{:?} ix old {}, new {}",
            ix,
            if old { "enabled" } else { "disabled" },
            if new { "enabled" } else { "disabled" }
        );
    }
}
