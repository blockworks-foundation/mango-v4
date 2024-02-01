use std::ops::DerefMut;

use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};

use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::health::*;
use crate::logs::{
    emit_perp_balances, emit_stack, PerpLiqBankruptcyLog, PerpLiqNegativePnlOrBankruptcyLog,
    TokenBalanceLog,
};
use crate::state::*;

pub fn perp_liq_negative_pnl_or_bankruptcy(
    ctx: Context<PerpLiqNegativePnlOrBankruptcyV2>,
    max_liab_transfer: u64,
) -> Result<()> {
    let mango_group = ctx.accounts.group.key();

    let now_slot = Clock::get()?.slot;
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();

    let perp_market_index;
    let settle_token_index;
    let perp_oracle_price;
    let settle_token_oracle_price;
    let insurance_token_oracle_price;
    {
        let perp_market = ctx.accounts.perp_market.load()?;
        perp_market_index = perp_market.perp_market_index;
        settle_token_index = perp_market.settle_token_index;
        let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
        perp_oracle_price = perp_market
            .oracle_price(&OracleAccountInfos::from_reader(oracle_ref), Some(now_slot))?;

        let settle_bank = ctx.accounts.settle_bank.load()?;
        let settle_oracle_ref = &AccountInfoRef::borrow(ctx.accounts.settle_oracle.as_ref())?;
        settle_token_oracle_price = settle_bank.oracle_price(
            &OracleAccountInfos::from_reader(settle_oracle_ref),
            Some(now_slot),
        )?;
        drop(settle_bank); // could be the same as insurance_bank

        let insurance_bank = ctx.accounts.insurance_bank.load()?;
        let insurance_oracle_ref = &AccountInfoRef::borrow(ctx.accounts.insurance_oracle.as_ref())?;
        // We're not getting the insurance token price from the HealthCache because
        // the liqee isn't guaranteed to have an insurance fund token position.
        insurance_token_oracle_price = insurance_bank.oracle_price(
            &OracleAccountInfos::from_reader(insurance_oracle_ref),
            Some(now_slot),
        )?;
    }

    require_keys_neq!(ctx.accounts.liqor.key(), ctx.accounts.liqee.key());
    let mut liqee = ctx.accounts.liqee.load_full_mut()?;
    let mut liqor = ctx.accounts.liqor.load_full_mut()?;
    // account constraint #1
    require!(
        liqor
            .fixed
            .is_owner_or_delegate(ctx.accounts.liqor_owner.key()),
        MangoError::SomeError
    );
    require_msg_typed!(
        !liqor.fixed.being_liquidated(),
        MangoError::BeingLiquidated,
        "liqor account"
    );

    let retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, &mango_group)
        .context("create account retriever")?;
    let mut liqee_health_cache = new_health_cache(&liqee.borrow(), &retriever, now_ts)?;
    drop(retriever);
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);

    // Guarantees that perp base position is 0 and perp quote position is <= 0.
    liqee_health_cache.require_after_phase2_liquidation()?;

    if liqee.check_liquidatable(&liqee_health_cache)? != CheckLiquidatable::Liquidatable {
        return Ok(());
    }

    // check positions exist/create them, done early for nicer error messages
    {
        liqee.perp_position(perp_market_index)?;
        liqee.token_position(settle_token_index)?;
        liqor.ensure_perp_position(perp_market_index, settle_token_index)?;
        liqor.ensure_token_position(settle_token_index)?;
    }

    let mut perp_market = ctx.accounts.perp_market.load_mut()?;

    let (settlement, insurance_transfer) = {
        let mut settle_bank = ctx.accounts.settle_bank.load_mut()?;
        let mut insurance_bank_opt =
            if ctx.accounts.settle_bank.key() != ctx.accounts.insurance_bank.key() {
                Some(ctx.accounts.insurance_bank.load_mut()?)
            } else {
                None
            };
        liquidation_action(
            ctx.accounts.group.key(),
            &mut perp_market,
            perp_oracle_price,
            &mut settle_bank,
            settle_token_oracle_price,
            insurance_bank_opt.as_mut().map(|v| v.deref_mut()),
            insurance_token_oracle_price,
            &ctx.accounts.insurance_vault,
            &mut liqor.borrow_mut(),
            ctx.accounts.liqor.key(),
            &mut liqee.borrow_mut(),
            ctx.accounts.liqee.key(),
            &mut liqee_health_cache,
            liqee_liq_end_health,
            now_ts,
            max_liab_transfer,
        )?
    };

    // Execute the insurance fund transfer if needed
    if insurance_transfer > 0 {
        let group = ctx.accounts.group.load()?;
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            insurance_transfer,
        )?;
    }

    //
    // Log positions afterwards
    //
    if settlement > 0 {
        let settle_bank = ctx.accounts.settle_bank.load()?;
        let liqor_token_position = liqor.token_position(settle_token_index)?;
        emit_stack(TokenBalanceLog {
            mango_group,
            mango_account: ctx.accounts.liqor.key(),
            token_index: settle_token_index,
            indexed_position: liqor_token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });

        let liqee_token_position = liqee.token_position(settle_token_index)?;
        emit_stack(TokenBalanceLog {
            mango_group,
            mango_account: ctx.accounts.liqee.key(),
            token_index: settle_token_index,
            indexed_position: liqee_token_position.indexed_position.to_bits(),
            deposit_index: settle_bank.deposit_index.to_bits(),
            borrow_index: settle_bank.borrow_index.to_bits(),
        });
    }

    if insurance_transfer > 0 {
        let insurance_bank = ctx.accounts.insurance_bank.load()?;
        let liqor_token_position = liqor.token_position(insurance_bank.token_index)?;
        emit_stack(TokenBalanceLog {
            mango_group,
            mango_account: ctx.accounts.liqor.key(),
            token_index: insurance_bank.token_index,
            indexed_position: liqor_token_position.indexed_position.to_bits(),
            deposit_index: insurance_bank.deposit_index.to_bits(),
            borrow_index: insurance_bank.borrow_index.to_bits(),
        });
    }

    let liqee_perp_position = liqee.perp_position(perp_market_index)?;
    let liqor_perp_position = liqor.perp_position(perp_market_index)?;
    emit_perp_balances(
        mango_group,
        ctx.accounts.liqor.key(),
        liqor_perp_position,
        &perp_market,
    );
    emit_perp_balances(
        mango_group,
        ctx.accounts.liqee.key(),
        liqee_perp_position,
        &perp_market,
    );

    // Check liqee health again: bankruptcy would improve health
    liqee_health_cache.recompute_perp_info(liqee_perp_position, &perp_market)?;
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_liq_end_health);

    drop(perp_market);

    // Check liqor's health
    if !liqor.fixed.is_in_health_region() {
        let account_retriever =
            ScanningAccountRetriever::new(ctx.remaining_accounts, &mango_group)?;
        let liqor_health = compute_health(
            &liqor.borrow(),
            HealthType::Init,
            &account_retriever,
            now_ts,
        )
        .context("compute liqor health")?;
        require!(liqor_health >= 0, MangoError::HealthMustBePositive);
    }

    Ok(())
}

pub(crate) fn liquidation_action(
    group_key: Pubkey,
    perp_market: &mut PerpMarket,
    perp_oracle_price: I80F48,
    settle_bank: &mut Bank,
    settle_token_oracle_price: I80F48,
    insurance_bank_opt: Option<&mut Bank>,
    insurance_token_oracle_price: I80F48,
    insurance_vault: &TokenAccount,
    liqor: &mut MangoAccountRefMut,
    liqor_key: Pubkey,
    liqee: &mut MangoAccountRefMut,
    liqee_key: Pubkey,
    liqee_health_cache: &mut HealthCache,
    liqee_liq_end_health: I80F48,
    now_ts: u64,
    max_liab_transfer: u64,
) -> Result<(I80F48, u64)> {
    let perp_market_index = perp_market.perp_market_index;
    let settle_token_index = perp_market.settle_token_index;
    let liqee_max_settle = liqee_health_cache.perp_max_settle(settle_token_index)?;
    let liqee_health_token_balances =
        liqee_health_cache.effective_token_balances(HealthType::LiquidationEnd);

    //
    // Step 1: Allow the liqor to take over ("settle") negative liqee pnl.
    //
    // The only limitation is the liqee's perp_max_settle and its perp pnl settle limit.
    // This does not change liqee health.
    //
    let settlement;
    let max_settlement_liqee;
    let mut liqee_pnl;
    {
        let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;
        let liqor_perp_position = liqor.perp_position_mut(perp_market_index)?;
        liqee_perp_position.settle_funding(&perp_market);
        liqor_perp_position.settle_funding(&perp_market);

        liqee_pnl = liqee_perp_position.unsettled_pnl(&perp_market, perp_oracle_price)?;
        require_gt!(0, liqee_pnl, MangoError::ProfitabilityMismatch);

        // Get settleable pnl on the liqee
        liqee_perp_position.update_settle_limit(&perp_market, now_ts);
        let liqee_settleable_pnl =
            liqee_perp_position.apply_pnl_settle_limit(&perp_market, liqee_pnl);

        max_settlement_liqee = liqee_max_settle
            .min(-liqee_settleable_pnl)
            .max(I80F48::ZERO);
        settlement = max_settlement_liqee
            .min(I80F48::from(max_liab_transfer))
            .max(I80F48::ZERO);
        if settlement > 0 {
            liqor_perp_position.record_liquidation_quote_change(-settlement);
            liqee_perp_position.record_settle(-settlement, &perp_market);

            // Update the accounts' perp_spot_transfer statistics.
            let settlement_i64 = settlement.round_to_zero().to_num::<i64>();
            liqor_perp_position.perp_spot_transfers += settlement_i64;
            liqee_perp_position.perp_spot_transfers -= settlement_i64;
            liqor.fixed.perp_spot_transfers += settlement_i64;
            liqee.fixed.perp_spot_transfers -= settlement_i64;

            // Transfer token balance
            let liqor_token_position = liqor.token_position_mut(settle_token_index)?.0;
            let liqee_token_position = liqee.token_position_mut(settle_token_index)?.0;
            settle_bank.deposit(liqor_token_position, settlement, now_ts)?;
            settle_bank.withdraw_without_fee(liqee_token_position, settlement, now_ts)?;
            liqee_health_cache.adjust_token_balance(&settle_bank, -settlement)?;

            emit_stack(PerpLiqNegativePnlOrBankruptcyLog {
                mango_group: group_key,
                liqee: liqee_key,
                liqor: liqor_key,
                perp_market_index,
                settlement: settlement.to_bits(),
            });

            liqee_pnl += settlement;
            msg!("liquidated pnl = {}", settlement);
        }
    };
    let max_liab_transfer = I80F48::from(max_liab_transfer) - settlement;

    // Step 2: bankruptcy
    //
    // If the liqee still has negative pnl and couldn't possibly be settled further, allow bankruptcy
    // to reduce the negative pnl.
    //
    // Remaining pnl that brings the account into negative init health is either:
    // - taken by the liqor in exchange for spot from the insurance fund, or
    // - wiped away and socialized among all perp participants (this does not involve the liqor)
    //
    let insurance_transfer;
    if settlement == max_settlement_liqee && liqee_pnl < 0 {
        // Preparation that's needed for both, insurance fund based pnl takeover and socialized loss

        let liqee_settle_token_balance = liqee_health_token_balances
            [liqee_health_cache.token_info_index(settle_token_index)?]
        .spot_and_perp;
        let liqee_perp_position = liqee.perp_position_mut(perp_market_index)?;

        // recompute for safety
        liqee_pnl = liqee_perp_position.unsettled_pnl(&perp_market, perp_oracle_price)?;

        // Each unit of pnl increase (towards 0) increases health, but the amount depends on whether
        // the health token position is negative or positive.
        // Compute how much pnl would need to be increased to reach liq end health 0 (while ignoring
        // liqee_pnl and other constraints initially, those are applied below)
        let max_for_health = {
            let liab_weighted_price = settle_token_oracle_price * settle_bank.init_liab_weight;
            let asset_weighted_price = settle_token_oracle_price * settle_bank.init_asset_weight;
            spot_amount_given_for_health_zero(
                liqee_liq_end_health,
                liqee_settle_token_balance,
                asset_weighted_price,
                liab_weighted_price,
            )?
        };

        let max_liab_transfer_from_liqee = (-liqee_pnl).min(max_for_health).max(I80F48::ZERO);

        let max_liab_transfer_to_liqor = max_liab_transfer_from_liqee
            .min(max_liab_transfer)
            .max(I80F48::ZERO);

        // Check if the insurance fund can be used to reimburse the liqor for taking on negative pnl

        // Available insurance fund coverage
        let insurance_vault_amount = if perp_market.elligible_for_group_insurance_fund() {
            insurance_vault.amount
        } else {
            0
        };

        let liquidation_fee_factor = I80F48::ONE + perp_market.base_liquidation_fee;
        let settle_token_price_with_fee = settle_token_oracle_price * liquidation_fee_factor;

        // Amount given to the liqor from the insurance fund
        insurance_transfer = (max_liab_transfer_to_liqor * settle_token_price_with_fee
            / insurance_token_oracle_price)
            .ceil()
            .to_num::<u64>()
            .min(insurance_vault_amount);

        let insurance_transfer_i80f48 = I80F48::from(insurance_transfer);
        let insurance_fund_exhausted = insurance_transfer == insurance_vault_amount;

        // Amount of negative perp pnl transfered to the liqor
        let insurance_liab_transfer = (insurance_transfer_i80f48 * insurance_token_oracle_price
            / settle_token_price_with_fee)
            .min(max_liab_transfer_to_liqor);

        // Try using the insurance fund if possible
        if insurance_transfer > 0 {
            let insurance_bank = insurance_bank_opt.unwrap_or(settle_bank);
            require_keys_eq!(insurance_bank.mint, insurance_vault.mint);

            // moving insurance assets into the insurance bank vault happens outside
            // of this function to ensure this is unittestable!

            // credit the liqor with quote tokens
            let (liqor_quote, _, _) = liqor.ensure_token_position(insurance_bank.token_index)?;
            insurance_bank.deposit(liqor_quote, insurance_transfer_i80f48, now_ts)?;

            // transfer perp quote loss from the liqee to the liqor
            let liqor_perp_position = liqor.perp_position_mut(perp_market_index)?;
            liqee_perp_position.record_settle(-insurance_liab_transfer, &perp_market);
            liqor_perp_position.record_liquidation_quote_change(-insurance_liab_transfer);

            msg!(
                "bankruptcy: {} pnl for {} insurance",
                insurance_liab_transfer,
                insurance_transfer
            );
        }

        // Socialize loss if the insurance fund is exhausted

        // At this point, we don't care about the liqor's requested max_liab_tranfer
        let remaining_liab = max_liab_transfer_from_liqee - insurance_liab_transfer;
        let mut socialized_loss = I80F48::ZERO;
        let (starting_long_funding, starting_short_funding) =
            (perp_market.long_funding, perp_market.short_funding);
        if insurance_fund_exhausted && remaining_liab > 0 {
            perp_market.socialize_loss(-remaining_liab)?;
            liqee_perp_position.record_settle(-remaining_liab, &perp_market);
            socialized_loss = remaining_liab;
            msg!("socialized loss: {}", socialized_loss);
        }

        emit_stack(PerpLiqBankruptcyLog {
            mango_group: group_key,
            liqee: liqee_key,
            liqor: liqor_key,
            perp_market_index: perp_market.perp_market_index,
            insurance_transfer: insurance_transfer_i80f48.to_bits(),
            socialized_loss: socialized_loss.to_bits(),
            starting_long_funding: starting_long_funding.to_bits(),
            starting_short_funding: starting_short_funding.to_bits(),
            ending_long_funding: perp_market.long_funding.to_bits(),
            ending_short_funding: perp_market.short_funding.to_bits(),
        });
    } else {
        insurance_transfer = 0;
    };

    Ok((settlement, insurance_transfer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{self, test::*};

    #[derive(Clone)]
    struct TestSetup {
        group: Pubkey,
        insurance_bank: TestAccount<Bank>,
        settle_bank: TestAccount<Bank>,
        other_bank: TestAccount<Bank>,
        insurance_oracle: TestAccount<StubOracle>,
        settle_oracle: TestAccount<StubOracle>,
        other_oracle: TestAccount<StubOracle>,
        perp_market: TestAccount<PerpMarket>,
        perp_oracle: TestAccount<StubOracle>,
        liqee: MangoAccountValue,
        liqor: MangoAccountValue,
        insurance_vault: spl_token::state::Account,
    }

    impl TestSetup {
        fn new() -> Self {
            let group = Pubkey::new_unique();
            let (mut insurance_bank, insurance_oracle) =
                mock_bank_and_oracle(group, 0, 1.0, 0.0, 0.0);
            let (settle_bank, settle_oracle) = mock_bank_and_oracle(group, 1, 1.0, 0.0, 0.0);

            let (_bank3, perp_oracle) = mock_bank_and_oracle(group, 4, 1.0, 0.5, 0.3);
            let mut perp_market =
                mock_perp_market(group, perp_oracle.pubkey, 1.0, 9, (0.2, 0.1), (0.2, 0.1));
            perp_market.data().settle_token_index = 1;
            perp_market.data().base_lot_size = 1;
            perp_market.data().group_insurance_fund = 1;

            let (other_bank, other_oracle) = mock_bank_and_oracle(group, 2, 1.0, 0.0, 0.0);

            let liqee_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqee = MangoAccountValue::from_bytes(&liqee_buffer).unwrap();
            {
                liqee.ensure_token_position(1).unwrap();
                liqee.ensure_token_position(2).unwrap();
                liqee.ensure_perp_position(9, 1).unwrap();
            }

            let liqor_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqor = MangoAccountValue::from_bytes(&liqor_buffer).unwrap();
            {
                liqor.ensure_token_position(0).unwrap();
                liqor.ensure_token_position(1).unwrap();
                liqor.ensure_perp_position(9, 1).unwrap();
            }

            let mut insurance_vault = spl_token::state::Account::default();
            insurance_vault.state = spl_token::state::AccountState::Initialized;
            insurance_vault.mint = insurance_bank.data().mint;

            Self {
                group,
                insurance_bank,
                settle_bank,
                other_bank,
                insurance_oracle,
                settle_oracle,
                other_oracle,
                perp_market,
                perp_oracle,
                liqee,
                liqor,
                insurance_vault,
            }
        }

        fn run(&self, max_liab_transfer: u64) -> Result<Self> {
            let mut setup = self.clone();

            let mut liqee_health_cache;
            let liqee_liq_end_health;
            {
                let ais = vec![
                    setup.insurance_bank.as_account_info(),
                    setup.settle_bank.as_account_info(),
                    setup.other_bank.as_account_info(),
                    setup.insurance_oracle.as_account_info(),
                    setup.settle_oracle.as_account_info(),
                    setup.other_oracle.as_account_info(),
                    setup.perp_market.as_account_info(),
                    setup.perp_oracle.as_account_info(),
                ];
                let retriever =
                    ScanningAccountRetriever::new_with_staleness(&ais, &setup.group, None).unwrap();

                liqee_health_cache =
                    health::new_health_cache(&setup.liqee.borrow(), &retriever, 0).unwrap();
                liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
            }

            let insurance_price = {
                let insurance_oracle_ai = setup.insurance_oracle.as_account_info();
                let insurance_oracle_ref = &AccountInfoRef::borrow(&insurance_oracle_ai)?;
                setup
                    .insurance_bank
                    .data()
                    .oracle_price(&OracleAccountInfos::from_reader(insurance_oracle_ref), None)
                    .unwrap()
            };
            let settle_price = {
                let settle_oracle_ai = setup.settle_oracle.as_account_info();
                let settle_oracle_ref = &AccountInfoRef::borrow(&settle_oracle_ai)?;
                setup
                    .settle_bank
                    .data()
                    .oracle_price(&OracleAccountInfos::from_reader(settle_oracle_ref), None)
                    .unwrap()
            };
            let perp_price = {
                let perp_oracle_ai = setup.perp_oracle.as_account_info();
                let perp_oracle_ref = &AccountInfoRef::borrow(&perp_oracle_ai)?;
                setup
                    .perp_market
                    .data()
                    .oracle_price(&OracleAccountInfos::from_reader(perp_oracle_ref), None)
                    .unwrap()
            };

            // There's no way to construct a TokenAccount directly...
            let mut buffer = [0u8; 165];
            use solana_program::program_pack::Pack;
            setup.insurance_vault.pack_into_slice(&mut buffer);
            let insurance_vault =
                TokenAccount::try_deserialize_unchecked(&mut &buffer[..]).unwrap();

            liquidation_action(
                setup.group.key(),
                setup.perp_market.data(),
                perp_price,
                setup.settle_bank.data(),
                settle_price,
                Some(setup.insurance_bank.data()),
                insurance_price,
                &insurance_vault,
                &mut setup.liqor.borrow_mut(),
                Pubkey::new_unique(),
                &mut setup.liqee.borrow_mut(),
                Pubkey::new_unique(),
                &mut liqee_health_cache,
                liqee_liq_end_health,
                0,
                max_liab_transfer,
            )?;

            Ok(setup)
        }
    }

    fn insurance_p(account: &mut MangoAccountValue) -> &mut TokenPosition {
        account.token_position_mut(0).unwrap().0
    }
    fn settle_p(account: &mut MangoAccountValue) -> &mut TokenPosition {
        account.token_position_mut(1).unwrap().0
    }
    fn other_p(account: &mut MangoAccountValue) -> &mut TokenPosition {
        account.token_position_mut(2).unwrap().0
    }

    fn perp_p(account: &mut MangoAccountValue) -> &mut PerpPosition {
        account.perp_position_mut(9).unwrap()
    }

    macro_rules! assert_eq_f {
        ($value:expr, $expected:expr, $max_error:expr) => {
            let value = $value;
            let expected = $expected;
            let ok = (value.to_num::<f64>() - expected).abs() < $max_error;
            assert!(ok, "value: {value}, expected: {expected}");
        };
    }

    #[test]
    fn test_liq_negative_pnl_or_bankruptcy() {
        let test_cases = vec![
            (
                "nothing",
                (0.9, 1.0, 1.0),
                (0.0, 0.0, 0.0, 0, 0),
                (false, 0.0, 0.0),
                (0.0, 0.0, 0.0, 0.0),
                100,
            ),
            (
                "settle 1",
                (0.9, 2.0, 3.0),
                // perp_settle_health = 40 * 2.0 * 0.9 - 36 = 36
                // max settle = 36 / (0.9 * 2.0) = 20
                (40.0, -50.0, -36.0, 6, 100),
                (true, 30.0, -40.0),
                (10.0, -10.0, 0.0, 0.0),
                10,
            ),
            (
                "settle 2 (+insurance)",
                (0.9, 2.0, 3.0),
                // perp_settle_health = 40 * 2.0 * 0.9 - 36 = 36
                // max settle = 36 / (0.9 * 2.0) = 20
                (40.0, -50.0, -36.0, 100, 100),
                (true, 20.0, -21.0),
                (20.0, -29.0, 6.0, 0.0),
                29, // limited by max_liab_transfer
            ),
            (
                "settle 3 (+insurance)",
                (0.9, 2.0, 3.0),
                // perp_settle_health = 40 * 2.0 * 0.9 - 36 = 36
                // max settle = 36 / (0.9 * 2.0) = 20
                (40.0, -50.0, -36.0, 100, 10), // limited by settleable pnl
                (true, 30.0, -31.0),
                (10.0, -19.0, 6.0, 0.0),
                19,
            ),
            (
                "settle 4 (+socialized loss)",
                (0.9, 2.0, 3.0),
                // perp_settle_health = 5 * 2.0 * 0.9 + 30 = 39
                // max settle = 5 + (39 - 9) / (2*1.1) = 18.63
                (5.0, -20.0, 30.0, 0, 100),
                (true, -13.63, 0.0),
                (18.63, -18.63, 0.0, 1.36), // socialized loss
                100,
            ),
            (
                "bankruptcy, no insurance 1",
                (0.9, 2.0, 1.0),
                (0.0, -5.0, 2.2, 0, 0),
                (true, 0.0, -1.0), // -1 * 2.0 * 1.1 = 2.2
                (0.0, 0.0, 0.0, 4.0),
                0,
            ),
            (
                "bankruptcy, no insurance 2",
                (0.9, 2.0, 1.0),
                (4.0, -5.0, -3.6, 0, 0), // health token balance goes from -1 to +2
                (true, 4.0, -2.0),
                (0.0, 0.0, 0.0, 3.0),
                0,
            ),
            (
                "bankruptcy, no insurance 3",
                (0.9, 2.0, 1.0),
                (4.0, -5.0, -3.6, 0, 0),
                (true, 4.0, -2.0),
                (0.0, 0.0, 0.0, 3.0),
                100, // liqor being willing to take over changes nothing
            ),
            (
                "bankruptcy, with insurance 1",
                (0.9, 2.0, 3.0),
                (40.0, -50.0, -36.0, 6, 0),
                (true, 40.0, -20.0),
                (0.0, -9.0, 6.0, 21.0), // 6 * 3.0 / 2.0 = 9 taken over, rest socialized loss
                100,
            ),
            (
                "bankruptcy, with insurance 2",
                (0.9, 2.0, 3.0),
                (40.0, -50.0, -36.0, 6, 0),
                (true, 40.0, -47.0),
                (0.0, -3.0, 2.0, 0.0),
                3, // liqor is limited, don't socialize loss since insurance not exhausted!
            ),
            (
                "bankruptcy, with insurance 3",
                (0.9, 2.0, 3.0),
                (40.0, -50.0, -36.0, 1000, 0), // insurance fund is big enough to cover fully
                (true, 40.0, -20.0),
                (0.0, -30.0, 20.0, 0.0),
                100,
            ),
            (
                "everything 1",
                (0.9, 2.0, 3.0),
                // perp_settle_health = 40 * 2.0 * 0.9 - 36 = 36
                // max settle = 36 / (0.9 * 2.0) = 20
                (40.0, -50.0, -36.0, 6, 100),
                (true, 20.0, 0.0),
                (20.0, -29.0, 6.0, 21.0),
                40,
            ),
            (
                "everything 2",
                (0.9, 2.0, 3.0),
                // perp_settle_health = 10 * 2.0 * 0.9 + 9 = 19
                // max settle = 10 + 9 / (1.1 * 2.0) = 14.1
                (10.0, -50.0, 9.0, 6, 100),
                (true, -4.1, 0.0), // perp position always goes to 0 because we use the same weights for init and maint
                (14.1, -14.1 - 9.0, 6.0, 50.0 - 14.1 - 9.0),
                40,
            ),
        ];

        for (
            name,
            (settle_token_weight, settle_price, insurance_price),
            // starting position
            (init_settle_token, init_perp, init_other, insurance_amount, settle_limit),
            // the expected liqee end position
            (exp_success, exp_liqee_settle_token, exp_liqee_perp),
            // expected liqor end position
            (exp_liqor_settle_token, exp_liqor_perp, exp_liqor_insurance, exp_socialized_loss),
            // maximum liquidation the liqor requests
            max_liab_transfer,
        ) in test_cases
        {
            println!("test: {name}");
            let mut setup = TestSetup::new();
            {
                let t = setup.settle_bank.data();
                t.init_asset_weight = I80F48::from_num(settle_token_weight);
                t.init_liab_weight = I80F48::from_num(2.0 - settle_token_weight);
                // maint weights used for perp settle health
                t.maint_asset_weight = I80F48::from_num(settle_token_weight);
                t.maint_liab_weight = I80F48::from_num(2.0 - settle_token_weight);
                t.stable_price_model.stable_price = settle_price;
                setup.settle_oracle.data().price = I80F48::from_num(settle_price);

                let t = setup.insurance_bank.data();
                t.stable_price_model.stable_price = insurance_price;
                setup.insurance_oracle.data().price = I80F48::from_num(insurance_price);

                let p = setup.perp_market.data();
                p.init_overall_asset_weight = I80F48::from_num(0.0);
                p.open_interest = 1;

                setup.insurance_vault.amount = insurance_amount;
            }
            {
                let p = perp_p(&mut setup.liqee);
                p.quote_position_native = I80F48::from_num(init_perp);
                p.recurring_settle_pnl_allowance = (settle_limit as i64).abs();

                let settle_bank = setup.settle_bank.data();
                settle_bank
                    .change_without_fee(
                        settle_p(&mut setup.liqee),
                        I80F48::from_num(init_settle_token),
                        0,
                    )
                    .unwrap();

                let other_bank = setup.other_bank.data();
                other_bank
                    .change_without_fee(other_p(&mut setup.liqee), I80F48::from_num(init_other), 0)
                    .unwrap();
            }

            let result = setup.run(max_liab_transfer);
            if !exp_success {
                assert!(result.is_err());
                continue;
            }
            let mut result = result.unwrap();

            let settle_bank = result.settle_bank.data();
            assert_eq_f!(
                settle_p(&mut result.liqee).native(settle_bank),
                exp_liqee_settle_token,
                0.01
            );
            assert_eq_f!(
                settle_p(&mut result.liqor).native(settle_bank),
                exp_liqor_settle_token,
                0.01
            );

            let insurance_bank = result.insurance_bank.data();
            assert_eq_f!(
                insurance_p(&mut result.liqor).native(insurance_bank),
                exp_liqor_insurance,
                0.01
            );

            assert_eq_f!(
                perp_p(&mut result.liqee).quote_position_native,
                exp_liqee_perp,
                0.1
            );
            assert_eq_f!(
                perp_p(&mut result.liqor).quote_position_native,
                exp_liqor_perp,
                0.1
            );

            assert_eq_f!(
                result.perp_market.data().long_funding,
                exp_socialized_loss,
                0.1
            );
        }
    }
}
