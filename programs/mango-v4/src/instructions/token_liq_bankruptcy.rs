use anchor_lang::prelude::*;
use anchor_spl::token;
use fixed::types::I80F48;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;

use crate::accounts_ix::*;
use crate::logs::{
    emit_stack, LoanOriginationFeeInstruction, TokenBalanceLog, TokenLiqBankruptcyLog,
    WithdrawLoanLog,
};

pub fn token_liq_bankruptcy(
    ctx: Context<TokenLiqBankruptcy>,
    max_liab_transfer: I80F48,
) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_pk = &ctx.accounts.group.key();

    // split remaining accounts into banks and health
    let liab_mint_info = ctx.accounts.liab_mint_info.load()?;
    let liab_token_index = liab_mint_info.token_index;
    let (bank_ais, health_ais) = &ctx.remaining_accounts.split_at(liab_mint_info.num_banks());
    liab_mint_info.verify_banks_ais(bank_ais)?;

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

    let mut account_retriever = ScanningAccountRetriever::new(health_ais, group_pk)?;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut liqee = ctx.accounts.liqee.load_full_mut()?;
    let mut liqee_health_cache = new_health_cache(&liqee.borrow(), &account_retriever, now_ts)
        .context("create liqee health cache")?;
    liqee_health_cache.require_after_phase2_liquidation()?;
    liqee.fixed.set_being_liquidated(true);

    let liab_is_insurance_token = liab_token_index == INSURANCE_TOKEN_INDEX;
    let (liab_bank, liab_oracle_price, opt_quote_bank_and_price) =
        account_retriever.banks_mut_and_oracles(liab_token_index, INSURANCE_TOKEN_INDEX)?;
    assert!(liab_is_insurance_token == opt_quote_bank_and_price.is_none());

    let mut liab_deposit_index = liab_bank.deposit_index;
    let liab_borrow_index = liab_bank.borrow_index;
    let (liqee_liab, liqee_raw_token_index) = liqee.token_position_mut(liab_token_index)?;
    let initial_liab_native = liqee_liab.native(liab_bank);

    let liqee_health_token_balances =
        liqee_health_cache.effective_token_balances(HealthType::LiquidationEnd);
    let liqee_liab_health_balance = liqee_health_token_balances
        [liqee_health_cache.token_info_index(liab_token_index)?]
    .spot_and_perp;

    // Allow token bankruptcy only while the spot position and health token position are both negative.
    // In particular, a very negative perp hupnl does not allow token bankruptcy to happen,
    // and if the perp hupnl is positive, we need to liquidate that before dealing with token
    // bankruptcy!
    require_gt!(I80F48::ZERO, initial_liab_native);
    require_gt!(I80F48::ZERO, liqee_liab_health_balance);
    // guaranteed positive
    let mut remaining_liab_loss = (-initial_liab_native).min(-liqee_liab_health_balance);

    // We pay for the liab token in quote. Example: SOL is at $20 and USDC is at $2, then for a liab
    // of 3 SOL, we'd pay 3 * 20 / 2 * (1+fee) = 30 * (1+fee) USDC.
    let liab_to_quote_with_fee =
        if let Some((_quote_bank, quote_price)) = opt_quote_bank_and_price.as_ref() {
            liab_oracle_price * (I80F48::ONE + liab_bank.liquidation_fee) / quote_price
        } else {
            I80F48::ONE
        };

    let liab_transfer_unrounded = remaining_liab_loss.min(max_liab_transfer);

    let insurance_vault_amount = if liab_mint_info.elligible_for_group_insurance_fund() {
        ctx.accounts.insurance_vault.amount
    } else {
        0
    };

    let insurance_transfer = (liab_transfer_unrounded * liab_to_quote_with_fee)
        .ceil()
        .to_num::<u64>()
        .min(insurance_vault_amount);

    let insurance_fund_exhausted = insurance_transfer == insurance_vault_amount;

    let insurance_transfer_i80f48 = I80F48::from(insurance_transfer);

    // AUDIT: v3 does this, but it seems bad, because it can make liab_transfer
    // exceed max_liab_transfer due to the ceil() above! Otoh, not doing it would allow
    // liquidators to exploit the insurance fund for 1 native token each call.
    let liab_transfer = insurance_transfer_i80f48 / liab_to_quote_with_fee;

    let mut liqee_liab_active = true;
    if insurance_transfer > 0 {
        // liqee gets liab assets (enable dusting to prevent a case where the position is brought
        // to +I80F48::DELTA)
        liqee_liab_active = liab_bank.deposit_with_dusting(liqee_liab, liab_transfer, now_ts)?;
        // update correctly even if dusting happened
        remaining_liab_loss -= liqee_liab.native(liab_bank) - initial_liab_native;

        // move insurance assets into quote bank
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            insurance_transfer,
        )?;

        // move quote assets into liqor and withdraw liab assets
        if let Some((quote_bank, _)) = opt_quote_bank_and_price {
            // account constraint #2 a)
            require_keys_eq!(quote_bank.vault, ctx.accounts.quote_vault.key());
            require_keys_eq!(quote_bank.mint, ctx.accounts.insurance_vault.mint);

            let quote_deposit_index = quote_bank.deposit_index;
            let quote_borrow_index = quote_bank.borrow_index;

            // credit the liqor
            let (liqor_quote, liqor_quote_raw_token_index, _) =
                liqor.ensure_token_position(INSURANCE_TOKEN_INDEX)?;
            let liqor_quote_active =
                quote_bank.deposit(liqor_quote, insurance_transfer_i80f48, now_ts)?;

            // liqor quote
            emit_stack(TokenBalanceLog {
                mango_group: ctx.accounts.group.key(),
                mango_account: ctx.accounts.liqor.key(),
                token_index: INSURANCE_TOKEN_INDEX,
                indexed_position: liqor_quote.indexed_position.to_bits(),
                deposit_index: quote_deposit_index.to_bits(),
                borrow_index: quote_borrow_index.to_bits(),
            });

            // transfer liab from liqee to liqor
            let (liqor_liab, liqor_liab_raw_token_index, _) =
                liqor.ensure_token_position(liab_token_index)?;
            let liqor_liab_withdraw_result =
                liab_bank.withdraw_with_fee(liqor_liab, liab_transfer, now_ts)?;

            // liqor liab
            emit_stack(TokenBalanceLog {
                mango_group: ctx.accounts.group.key(),
                mango_account: ctx.accounts.liqor.key(),
                token_index: liab_token_index,
                indexed_position: liqor_liab.indexed_position.to_bits(),
                deposit_index: liab_deposit_index.to_bits(),
                borrow_index: liab_borrow_index.to_bits(),
            });

            // Check liqor's health
            if !liqor.fixed.is_in_health_region() {
                let liqor_health = compute_health(
                    &liqor.borrow(),
                    HealthType::Init,
                    &account_retriever,
                    now_ts,
                )?;
                require!(liqor_health >= 0, MangoError::HealthMustBePositive);
            }

            if liqor_liab_withdraw_result
                .loan_origination_fee
                .is_positive()
            {
                emit_stack(WithdrawLoanLog {
                    mango_group: ctx.accounts.group.key(),
                    mango_account: ctx.accounts.liqor.key(),
                    token_index: liab_token_index,
                    loan_amount: liqor_liab_withdraw_result.loan_amount.to_bits(),
                    loan_origination_fee: liqor_liab_withdraw_result.loan_origination_fee.to_bits(),
                    instruction: LoanOriginationFeeInstruction::LiqTokenBankruptcy,
                    price: Some(liab_oracle_price.to_bits()),
                });
            }

            if !liqor_quote_active {
                liqor.deactivate_token_position_and_log(
                    liqor_quote_raw_token_index,
                    ctx.accounts.liqor.key(),
                );
            }
            if !liqor_liab_withdraw_result.position_is_active {
                liqor.deactivate_token_position_and_log(
                    liqor_liab_raw_token_index,
                    ctx.accounts.liqor.key(),
                );
            }
        } else {
            // For liab_token_index == INSURANCE_TOKEN_INDEX: the insurance fund deposits directly into liqee,
            // without a fee or the liqor being involved
            // account constraint #2 b)
            require_keys_eq!(liab_bank.vault, ctx.accounts.quote_vault.key());
            require_eq!(liab_token_index, INSURANCE_TOKEN_INDEX);
            require_eq!(liab_to_quote_with_fee, I80F48::ONE);
            require_eq!(insurance_transfer_i80f48, liab_transfer);
        }
    }
    drop(account_retriever);

    // Socialize loss if there's more loss and noone else could use the
    // insurance fund to cover it.
    let mut socialized_loss = I80F48::ZERO;
    let starting_deposit_index = liab_deposit_index;
    if insurance_fund_exhausted && remaining_liab_loss.is_positive() {
        // find the total deposits
        let mut indexed_total_deposits = I80F48::ZERO;
        for bank_ai in bank_ais.iter() {
            let bank = bank_ai.load::<Bank>()?;
            indexed_total_deposits += bank.indexed_deposits;
        }

        // This is the solution to:
        //   total_indexed_deposits * (deposit_index - new_deposit_index) = remaining_liab_loss
        // AUDIT: Could it happen that remaining_liab_loss > total_indexed_deposits * deposit_index?
        //        Probably not.
        let new_deposit_index = liab_deposit_index - remaining_liab_loss / indexed_total_deposits;
        liab_deposit_index = new_deposit_index;
        socialized_loss = remaining_liab_loss;

        let mut amount_to_credit = remaining_liab_loss;
        for bank_ai in bank_ais.iter() {
            let mut bank = bank_ai.load_mut::<Bank>()?;
            bank.deposit_index = new_deposit_index;

            // credit liqee on each bank where we can offset borrows
            let amount_for_bank = amount_to_credit.min(bank.native_borrows());
            if amount_for_bank.is_positive() {
                // enable dusting, because each deposit() is allowed to round up. thus multiple deposit
                // could bring the total position slightly above zero otherwise
                liqee_liab_active =
                    bank.deposit_with_dusting(liqee_liab, amount_for_bank, now_ts)?;
                amount_to_credit -= amount_for_bank;
                if amount_to_credit <= 0 {
                    break;
                }
            }
        }

        // socialized loss always brings the position to zero
        require_eq!(liqee_liab.indexed_position, I80F48::ZERO);
    }

    // liqee liab
    emit_stack(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.liqee.key(),
        token_index: liab_token_index,
        indexed_position: liqee_liab.indexed_position.to_bits(),
        deposit_index: liab_deposit_index.to_bits(),
        borrow_index: liab_borrow_index.to_bits(),
    });

    let liab_bank = bank_ais[0].load::<Bank>()?;
    let end_liab_native = liqee_liab.native(&liab_bank);
    liqee_health_cache.adjust_token_balance(&liab_bank, end_liab_native - initial_liab_native)?;

    // Check liqee health again
    let liqee_liq_end_health = liqee_health_cache.health(HealthType::LiquidationEnd);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_liq_end_health);

    if !liqee_liab_active {
        liqee.deactivate_token_position_and_log(liqee_raw_token_index, ctx.accounts.liqee.key());
    }

    emit_stack(TokenLiqBankruptcyLog {
        mango_group: ctx.accounts.group.key(),
        liqee: ctx.accounts.liqee.key(),
        liqor: ctx.accounts.liqor.key(),
        liab_token_index,
        initial_liab_native: initial_liab_native.to_bits(),
        liab_price: liab_oracle_price.to_bits(),
        insurance_token_index: INSURANCE_TOKEN_INDEX,
        insurance_transfer: insurance_transfer_i80f48.to_bits(),
        socialized_loss: socialized_loss.to_bits(),
        starting_liab_deposit_index: starting_deposit_index.to_bits(),
        ending_liab_deposit_index: liab_deposit_index.to_bits(),
    });

    Ok(())
}
