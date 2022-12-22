use anchor_lang::prelude::*;
use fixed::types::I80F48;
use std::cmp::min;

use crate::accounts_ix::*;
use crate::error::*;
use crate::health::*;
use crate::logs::{
    LoanOriginationFeeInstruction, TokenBalanceLog, TokenLiqWithTokenLog,
    WithdrawLoanOriginationFeeLog,
};
use crate::state::*;
use crate::util::checked_math as cm;

pub fn token_liq_with_token(
    ctx: Context<TokenLiqWithToken>,
    asset_token_index: TokenIndex,
    liab_token_index: TokenIndex,
    max_liab_transfer: I80F48,
) -> Result<()> {
    let group_pk = &ctx.accounts.group.key();

    require!(asset_token_index != liab_token_index, MangoError::SomeError);
    let mut account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)
        .context("create account retriever")?;

    require_keys_neq!(ctx.accounts.liqor.key(), ctx.accounts.liqee.key());
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

    let mut liqee = ctx.accounts.liqee.load_full_mut()?;

    // Initial liqee health check
    let mut liqee_health_cache = new_health_cache(&liqee.borrow(), &account_retriever)
        .context("create liqee health cache")?;
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee_health_cache.require_after_phase1_liquidation()?;

    if !liqee.check_liquidatable(&liqee_health_cache)? {
        return Ok(());
    }

    //
    // Transfer some liab_token from liqor to liqee and
    // transfer some asset_token from liqee to liqor.
    //
    let now_ts = Clock::get()?.unix_timestamp.try_into().unwrap();
    liquidation_action(
        &mut account_retriever,
        liab_token_index,
        asset_token_index,
        &mut liqor.borrow_mut(),
        ctx.accounts.liqor.key(),
        &mut liqee.borrow_mut(),
        ctx.accounts.liqee.key(),
        &mut liqee_health_cache,
        liqee_liq_end_health,
        now_ts,
        max_liab_transfer,
    )?;

    // Check liqor's health
    if !liqor.fixed.is_in_health_region() {
        let liqor_health = compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)
            .context("compute liqor health")?;
        require!(liqor_health >= 0, MangoError::HealthMustBePositive);
    }

    Ok(())
}

pub(crate) fn liquidation_action(
    account_retriever: &mut ScanningAccountRetriever,
    liab_token_index: TokenIndex,
    asset_token_index: TokenIndex,
    liqor: &mut MangoAccountRefMut,
    liqor_key: Pubkey,
    liqee: &mut MangoAccountRefMut,
    liqee_key: Pubkey,
    liqee_health_cache: &mut HealthCache,
    liqee_liq_end_health: I80F48,
    now_ts: u64,
    max_liab_transfer: I80F48,
) -> Result<()> {
    // Get the mut banks and oracle prices
    //
    // This must happen _after_ the health computation, since immutable borrows of
    // the bank are not allowed at the same time.
    let (asset_bank, asset_oracle_price, opt_liab_bank_and_price) =
        account_retriever.banks_mut_and_oracles(asset_token_index, liab_token_index)?;
    let (liab_bank, liab_oracle_price) = opt_liab_bank_and_price.unwrap();

    // The main complication here is that we can't keep the liqee_asset_position and liqee_liab_position
    // borrows alive at the same time. Possibly adding get_mut_pair() would be helpful.
    let (liqee_asset_position, liqee_asset_raw_index) =
        liqee.token_position_and_raw_index(asset_token_index)?;
    let liqee_asset_native = liqee_asset_position.native(asset_bank);
    require!(liqee_asset_native.is_positive(), MangoError::SomeError);

    let (liqee_liab_position, liqee_liab_raw_index) =
        liqee.token_position_and_raw_index(liab_token_index)?;
    let liqee_liab_native = liqee_liab_position.native(liab_bank);
    require!(liqee_liab_native.is_negative(), MangoError::SomeError);

    // Liquidation fees work by giving the liqor more assets than the oracle price would
    // indicate. Specifically we choose
    //   assets =
    //     liabs * liab_oracle_price/asset_oracle_price * (1 + liab_liq_fee + asset_liq_fee)
    // Which means that we use a increased liab oracle price and reduced asset oracle price for
    // the conversion.
    // It would be more fully correct to use (1+liab_liq_fee)*(1+asset_liq_fee), but for small
    // fee amounts that is nearly identical.
    // For simplicity we write
    //   assets = liabs * liab_oracle_price / asset_oracle_price * fee_factor
    //   assets = liabs * liab_oracle_price_adjusted / asset_oracle_price
    //          = liabs * lopa / aop
    let fee_factor = cm!(I80F48::ONE + asset_bank.liquidation_fee + liab_bank.liquidation_fee);
    let liab_oracle_price_adjusted = cm!(liab_oracle_price * fee_factor);

    let init_asset_weight = asset_bank.init_asset_weight;
    let init_liab_weight = liab_bank.init_liab_weight;

    // The price the LiquidationEnd health computation uses for a liability of one native liab token
    let liab_liq_end_price = liqee_health_cache
        .token_info(liab_token_index)
        .unwrap()
        .prices
        .liab(HealthType::LiquidationEnd);
    // Health price for an asset of one native asset token
    let asset_liq_end_price = liqee_health_cache
        .token_info(asset_token_index)
        .unwrap()
        .prices
        .asset(HealthType::LiquidationEnd);

    // How much asset would need to be exchanged to liab in order to bring health to 0?
    //
    // That means: what is x (unit: native liab tokens) such that
    //   init_health
    //     + x * ilw * llep     health gain from reducing liabs
    //     - y * iaw * alep     health loss from paying asset
    //     = 0
    // where
    //   ilw = init_liab_weight,
    //   llep = liab_liq_end_price,
    //   lopa = liab_oracle_price_adjusted, (see above)
    //   iap = init_asset_weight,
    //   alep = asset_liq_end_price,
    //   aop = asset_oracle_price
    //   ff = fee_factor
    // and the asset cost of getting x native units of liab is:
    //   y = x * lopa / aop   (native asset tokens, see above)
    //
    // Result: x = -init_health / (ilw * llep - iaw * lopa * alep / aop)
    let liab_needed = cm!(-liqee_liq_end_health
        / (liab_liq_end_price * init_liab_weight
            - liab_oracle_price_adjusted
                * init_asset_weight
                * (asset_liq_end_price / asset_oracle_price)));

    // How much liab can we get at most for the asset balance?
    let liab_possible = cm!(liqee_asset_native * asset_oracle_price / liab_oracle_price_adjusted);

    // The amount of liab native tokens we will transfer
    let liab_transfer = min(
        min(min(liab_needed, -liqee_liab_native), liab_possible),
        max_liab_transfer,
    );

    // The amount of asset native tokens we will give up for them
    let asset_transfer = cm!(liab_transfer * liab_oracle_price_adjusted / asset_oracle_price);

    // During liquidation, we mustn't leave small positive balances in the liqee. Those
    // could break bankruptcy-detection. Thus we dust them even if the token position
    // is nominally in-use.

    // Apply the balance changes to the liqor and liqee accounts
    let liqee_liab_position = liqee.token_position_mut_by_raw_index(liqee_liab_raw_index);
    let liqee_liab_active =
        liab_bank.deposit_with_dusting(liqee_liab_position, liab_transfer, now_ts)?;
    let liqee_liab_indexed_position = liqee_liab_position.indexed_position;

    let (liqor_liab_position, liqor_liab_raw_index, _) =
        liqor.ensure_token_position(liab_token_index)?;
    let (liqor_liab_active, loan_origination_fee) = liab_bank.withdraw_with_fee(
        liqor_liab_position,
        liab_transfer,
        now_ts,
        liab_oracle_price,
    )?;
    let liqor_liab_indexed_position = liqor_liab_position.indexed_position;
    let liqee_liab_native_after = liqee_liab_position.native(liab_bank);

    let (liqor_asset_position, liqor_asset_raw_index, _) =
        liqor.ensure_token_position(asset_token_index)?;
    let liqor_asset_active = asset_bank.deposit(liqor_asset_position, asset_transfer, now_ts)?;
    let liqor_asset_indexed_position = liqor_asset_position.indexed_position;

    let liqee_asset_position = liqee.token_position_mut_by_raw_index(liqee_asset_raw_index);
    let liqee_asset_active = asset_bank.withdraw_without_fee_with_dusting(
        liqee_asset_position,
        asset_transfer,
        now_ts,
        asset_oracle_price,
    )?;
    let liqee_asset_indexed_position = liqee_asset_position.indexed_position;
    let liqee_assets_native_after = liqee_asset_position.native(asset_bank);

    // Update the health cache
    liqee_health_cache
        .adjust_token_balance(liab_bank, cm!(liqee_liab_native_after - liqee_liab_native))?;
    liqee_health_cache.adjust_token_balance(
        asset_bank,
        cm!(liqee_assets_native_after - liqee_asset_native),
    )?;

    msg!(
        "liquidated {} liab for {} asset",
        liab_transfer,
        asset_transfer
    );

    // liqee asset
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqee_key,
        token_index: asset_token_index,
        indexed_position: liqee_asset_indexed_position.to_bits(),
        deposit_index: asset_bank.deposit_index.to_bits(),
        borrow_index: asset_bank.borrow_index.to_bits(),
    });
    // liqee liab
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqee_key,
        token_index: liab_token_index,
        indexed_position: liqee_liab_indexed_position.to_bits(),
        deposit_index: liab_bank.deposit_index.to_bits(),
        borrow_index: liab_bank.borrow_index.to_bits(),
    });
    // liqor asset
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqor_key,
        token_index: asset_token_index,
        indexed_position: liqor_asset_indexed_position.to_bits(),
        deposit_index: asset_bank.deposit_index.to_bits(),
        borrow_index: asset_bank.borrow_index.to_bits(),
    });
    // liqor liab
    emit!(TokenBalanceLog {
        mango_group: liqee.fixed.group,
        mango_account: liqor_key,
        token_index: liab_token_index,
        indexed_position: liqor_liab_indexed_position.to_bits(),
        deposit_index: liab_bank.deposit_index.to_bits(),
        borrow_index: liab_bank.borrow_index.to_bits(),
    });

    if loan_origination_fee.is_positive() {
        emit!(WithdrawLoanOriginationFeeLog {
            mango_group: liqee.fixed.group,
            mango_account: liqor_key,
            token_index: liab_token_index,
            loan_origination_fee: loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::LiqTokenWithToken
        });
    }

    // Since we use a scanning account retriever, it's safe to deactivate inactive token positions
    if !liqee_asset_active {
        liqee.deactivate_token_position_and_log(liqee_asset_raw_index, liqee_key);
    }
    if !liqee_liab_active {
        liqee.deactivate_token_position_and_log(liqee_liab_raw_index, liqee_key);
    }
    if !liqor_asset_active {
        liqor.deactivate_token_position_and_log(liqor_asset_raw_index, liqor_key);
    }
    if !liqor_liab_active {
        liqor.deactivate_token_position_and_log(liqor_liab_raw_index, liqor_key)
    }

    // Check liqee health again
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_liq_end_health);

    emit!(TokenLiqWithTokenLog {
        mango_group: liqee.fixed.group,
        liqee: liqee_key,
        liqor: liqor_key,
        asset_token_index,
        liab_token_index,
        asset_transfer: asset_transfer.to_bits(),
        liab_transfer: liab_transfer.to_bits(),
        asset_price: asset_oracle_price.to_bits(),
        liab_price: liab_oracle_price.to_bits(),
        bankruptcy: !liqee_health_cache.has_phase2_liquidatable()
            & liqee_liq_end_health.is_negative()
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{self, test::*};

    #[derive(Clone)]
    struct TestSetup {
        group: Pubkey,
        asset_bank: TestAccount<Bank>,
        liab_bank: TestAccount<Bank>,
        asset_oracle: TestAccount<StubOracle>,
        liab_oracle: TestAccount<StubOracle>,
        liqee: MangoAccountValue,
        liqor: MangoAccountValue,
    }

    impl TestSetup {
        fn new() -> Self {
            let group = Pubkey::new_unique();
            let (asset_bank, asset_oracle) = mock_bank_and_oracle(group, 0, 1.0, 0.0, 0.0);
            let (liab_bank, liab_oracle) = mock_bank_and_oracle(group, 1, 1.0, 0.0, 0.0);

            let liqee_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqee = MangoAccountValue::from_bytes(&liqee_buffer).unwrap();
            {
                liqee.ensure_token_position(0).unwrap();
                liqee.ensure_token_position(1).unwrap();
            }

            let liqor_buffer = MangoAccount::default_for_tests().try_to_vec().unwrap();
            let mut liqor = MangoAccountValue::from_bytes(&liqor_buffer).unwrap();
            {
                liqor.ensure_token_position(0).unwrap();
                liqor.ensure_token_position(1).unwrap();
            }

            Self {
                group,
                asset_bank,
                liab_bank,
                asset_oracle,
                liab_oracle,
                liqee,
                liqor,
            }
        }

        fn liqee_health_cache(&self) -> HealthCache {
            let mut setup = self.clone();

            let ais = vec![
                setup.asset_bank.as_account_info(),
                setup.liab_bank.as_account_info(),
                setup.asset_oracle.as_account_info(),
                setup.liab_oracle.as_account_info(),
            ];
            let retriever =
                ScanningAccountRetriever::new_with_staleness(&ais, &setup.group, None).unwrap();

            health::new_health_cache(&setup.liqee.borrow(), &retriever).unwrap()
        }

        fn run(&self, max_liab_transfer: I80F48) -> Result<Self> {
            let mut setup = self.clone();

            let ais = vec![
                setup.asset_bank.as_account_info(),
                setup.liab_bank.as_account_info(),
                setup.asset_oracle.as_account_info(),
                setup.liab_oracle.as_account_info(),
            ];
            let mut retriever =
                ScanningAccountRetriever::new_with_staleness(&ais, &setup.group, None).unwrap();

            let mut liqee_health_cache =
                health::new_health_cache(&setup.liqee.borrow(), &retriever).unwrap();
            let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);

            liquidation_action(
                &mut retriever,
                1,
                0,
                &mut setup.liqor.borrow_mut(),
                Pubkey::new_unique(),
                &mut setup.liqee.borrow_mut(),
                Pubkey::new_unique(),
                &mut liqee_health_cache,
                liqee_liq_end_health,
                0,
                max_liab_transfer,
            )?;

            drop(retriever);
            drop(ais);

            Ok(setup)
        }
    }

    fn asset_p(account: &mut MangoAccountValue) -> &mut TokenPosition {
        account.token_position_mut(0).unwrap().0
    }
    fn liab_p(account: &mut MangoAccountValue) -> &mut TokenPosition {
        account.token_position_mut(1).unwrap().0
    }

    macro_rules! assert_eq_f {
        ($value:expr, $expected:expr, $max_error:expr) => {
            let value = $value;
            let expected = $expected;
            let ok = (value.to_num::<f64>() - expected).abs() < $max_error;
            assert!(ok, "value: {value}, expected: {expected}");
        };
    }

    // Check that stable price and weight scaling does not affect liquidation targets
    #[test]
    fn test_liq_with_token_stable_price() {
        let mut setup = TestSetup::new();
        {
            let ab = setup.asset_bank.data();
            ab.stable_price_model.stable_price = 0.5;
            ab.deposit_weight_scale_start_quote = 505.0;
            let lb = setup.liab_bank.data();
            lb.stable_price_model.stable_price = 1.25;
            lb.borrow_weight_scale_start_quote = 3.75;
            lb.init_liab_weight = I80F48::from_num(1.4);
            lb.maint_liab_weight = I80F48::from_num(1.2);
        }
        {
            let asset_bank = setup.asset_bank.data();
            asset_bank
                .change_without_fee(
                    asset_p(&mut setup.liqee),
                    I80F48::from_num(10.0),
                    0,
                    I80F48::from(1),
                )
                .unwrap();
            asset_bank
                .change_without_fee(
                    asset_p(&mut setup.liqor),
                    I80F48::from_num(1000.0),
                    0,
                    I80F48::from(1),
                )
                .unwrap();

            let liab_bank = setup.liab_bank.data();
            liab_bank
                .change_without_fee(
                    liab_p(&mut setup.liqor),
                    I80F48::from_num(1000.0),
                    0,
                    I80F48::from(1),
                )
                .unwrap();
            liab_bank
                .change_without_fee(
                    liab_p(&mut setup.liqee),
                    I80F48::from_num(-9.0),
                    0,
                    I80F48::from(1),
                )
                .unwrap();
        }

        let hc = setup.liqee_health_cache();
        let asset_scale = 505.0 / 1010.0;
        let liab_scale = 9.0 * 1.25 / 3.75;
        assert_eq_f!(
            hc.health(HealthType::Init),
            10.0 * 0.5 * asset_scale - 9.0 * 1.25 * 1.4 * liab_scale,
            0.1
        );
        assert_eq_f!(hc.health(HealthType::LiquidationEnd), 10.0 - 9.0 * 1.4, 0.1);
        assert_eq_f!(hc.health(HealthType::Maint), 10.0 - 9.0 * 1.2, 0.1);

        let mut result = setup.run(I80F48::from(100)).unwrap();

        let liqee_asset = asset_p(&mut result.liqee);
        assert_eq_f!(liqee_asset.native(&result.asset_bank.data()), 3.5, 0.01);
        let liqee_liab = liab_p(&mut result.liqee);
        assert_eq_f!(liqee_liab.native(&result.liab_bank.data()), -2.5, 0.01);

        let hc = result.liqee_health_cache();
        assert_eq_f!(hc.health(HealthType::LiquidationEnd), 0.0, 0.01);
    }
}
