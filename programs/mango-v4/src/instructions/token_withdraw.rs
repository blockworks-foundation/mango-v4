use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::health::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token;
use fixed::types::I80F48;

use crate::accounts_ix::*;
use crate::logs::{
    emit_stack, LoanOriginationFeeInstruction, TokenBalanceLog, WithdrawLoanLog, WithdrawLog,
};

pub fn token_withdraw(ctx: Context<TokenWithdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
    require_msg!(amount > 0, "withdraw amount must be positive");

    let group = ctx.accounts.group.load()?;
    let token_index = ctx.accounts.bank.load()?.token_index;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    // Create the account's position for that token index
    let mut account = ctx.accounts.account.load_full_mut()?;
    let (_, raw_token_index, _) = account.ensure_token_position(token_index)?;

    // Health check _after_ the token position is guaranteed to exist
    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let retriever =
            new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
        let hc_result = new_health_cache(&account.borrow(), &retriever, now_ts)
            .context("pre-withdraw health cache");
        if hc_result.is_oracle_error() {
            // We allow NOT checking the pre init health. That means later on the health
            // check will be stricter (post_init > 0, without the post_init >= pre_init option)
            // Then later we can compute the health while ignoring potential nonnegative
            // health contributions from tokens with stale oracles.
            None
        } else {
            let health_cache = hc_result?;
            let pre_init_health = account.check_health_pre(&health_cache)?;
            Some((health_cache, pre_init_health))
        }
    } else {
        None
    };

    let mut bank = ctx.accounts.bank.load_mut()?;
    let position = account.token_position_mut_by_raw_index(raw_token_index);
    let native_position = position.native(&bank);

    // Handle amount special case for withdrawing everything
    let amount = if amount == u64::MAX && !allow_borrow {
        if !native_position.is_negative() {
            // TODO: This rounding may mean that if we deposit and immediately withdraw
            //       we can't withdraw the full amount!
            native_position.floor().to_num::<u64>()
        } else {
            return Ok(());
        }
    } else {
        amount
    };

    let is_borrow = amount > native_position;
    require!(allow_borrow || !is_borrow, MangoError::SomeError);
    if bank.are_borrows_reduce_only() {
        require!(!is_borrow, MangoError::TokenInReduceOnlyMode);
    }

    let amount_i80f48 = I80F48::from(amount);

    // Get the oracle price, even if stale or unconfident: We want to allow users
    // to withdraw deposits (while staying healthy otherwise) if the oracle is bad.
    let unsafe_oracle_state = oracle_state_unchecked(
        &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?,
        bank.mint_decimals,
    )?;

    // Update the bank and position
    let withdraw_result = bank.withdraw_with_fee(
        position,
        amount_i80f48,
        Clock::get()?.unix_timestamp.try_into().unwrap(),
    )?;

    // Avoid getting in trouble because of the mutable bank account borrow later
    drop(bank);
    let bank = ctx.accounts.bank.load()?;

    // Provide a readable error message in case the vault doesn't have enough tokens
    if ctx.accounts.vault.amount < amount {
        return err!(MangoError::InsufficentBankVaultFunds).with_context(|| {
            format!(
                "bank vault does not have enough tokens, need {} but have {}",
                amount, ctx.accounts.vault.amount
            )
        });
    }

    // Transfer the actual tokens
    let group_seeds = group_seeds!(group);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    let native_position_after = position.native(&bank);

    emit_stack(TokenBalanceLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        token_index,
        indexed_position: position.indexed_position.to_bits(),
        deposit_index: bank.deposit_index.to_bits(),
        borrow_index: bank.borrow_index.to_bits(),
    });

    // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
    let amount_usd = (amount_i80f48 * unsafe_oracle_state.price).to_num::<i64>();
    account.fixed.net_deposits -= amount_usd;

    //
    // Health check
    //
    if !account.fixed.is_in_health_region() {
        if let Some((mut health_cache, pre_init_health)) = pre_health_opt {
            // This is the normal case
            health_cache.adjust_token_balance(&bank, native_position_after - native_position)?;
            account.check_health_post(&health_cache, pre_init_health)?;
        } else {
            // Some oracle was stale/not confident enough above.
            //
            // Try computing health while ignoring nonnegative contributions from bad oracles.
            // If the health is good enough without those, we can pass.
            //
            // Note that this must include the normal pre and post health checks.
            let retriever =
                new_fixed_order_account_retriever(ctx.remaining_accounts, &account.borrow())?;
            let health_cache =
                new_health_cache_skipping_bad_oracles(&account.borrow(), &retriever, now_ts)
                    .context("special post-withdraw health-cache")?;
            let post_init_health = health_cache.health(HealthType::Init);
            account.check_health_pre_checks(&health_cache, post_init_health)?;
            account.check_health_post_checks(I80F48::MAX, post_init_health)?;
        }
    }

    //
    // Deactivate the position only after the health check because the user passed in
    // remaining_accounts for all banks/oracles, including the account that will now be
    // deactivated.
    //
    if !withdraw_result.position_is_active {
        account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
    }

    emit_stack(WithdrawLog {
        mango_group: ctx.accounts.group.key(),
        mango_account: ctx.accounts.account.key(),
        signer: ctx.accounts.owner.key(),
        token_index,
        quantity: amount,
        price: unsafe_oracle_state.price.to_bits(),
    });

    if withdraw_result.loan_origination_fee.is_positive() {
        emit_stack(WithdrawLoanLog {
            mango_group: ctx.accounts.group.key(),
            mango_account: ctx.accounts.account.key(),
            token_index,
            loan_amount: withdraw_result.loan_amount.to_bits(),
            loan_origination_fee: withdraw_result.loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::TokenWithdraw,
            price: Some(unsafe_oracle_state.price.to_bits()),
        });
    }

    // Enforce min vault to deposits ratio and net borrow limits
    if is_borrow {
        ctx.accounts.vault.reload()?;
        bank.enforce_min_vault_to_deposits_ratio(ctx.accounts.vault.as_ref())?;

        // When borrowing the price has be trustworthy, so we can do a reasonable
        // net borrow check.
        unsafe_oracle_state.check_confidence_and_maybe_staleness(
            &bank.oracle,
            &bank.oracle_config,
            Some(Clock::get()?.slot),
        )?;
        bank.check_net_borrows(unsafe_oracle_state.price)?;
    }

    Ok(())
}
