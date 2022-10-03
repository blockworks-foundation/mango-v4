use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use fixed::types::I80F48;

use crate::error::*;
use crate::state::ScanningAccountRetriever;
use crate::state::*;
use crate::util::checked_math as cm;

use crate::logs::{emit_perp_balances, PerpLiqBankruptcyLog};

// Remaining accounts:
// - merged health accounts for liqor+liqee
#[derive(Accounts)]
pub struct PerpLiqBankruptcy<'info> {
    #[account(
        has_one = insurance_vault,
    )]
    pub group: AccountLoader<'info, Group>,

    #[account(mut, has_one = group)]
    pub perp_market: AccountLoader<'info, PerpMarket>,

    #[account(
        mut,
        has_one = group
        // liqor_owner is checked at #1
    )]
    pub liqor: AccountLoaderDynamic<'info, MangoAccount>,
    pub liqor_owner: Signer<'info>,

    #[account(
        mut,
        has_one = group
    )]
    pub liqee: AccountLoaderDynamic<'info, MangoAccount>,

    #[account(
        mut,
        has_one = group,
        constraint = quote_bank.load()?.vault == quote_vault.key()
        // address is checked at #2
    )]
    pub quote_bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub quote_vault: Account<'info, TokenAccount>,

    // future: this would be an insurance fund vault specific to a
    // trustless token, separate from the shared one on the group
    #[account(mut)]
    pub insurance_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> PerpLiqBankruptcy<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.insurance_vault.to_account_info(),
            to: self.quote_vault.to_account_info(),
            authority: self.group.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

pub fn perp_liq_bankruptcy(ctx: Context<PerpLiqBankruptcy>, max_liab_transfer: u64) -> Result<()> {
    let group = ctx.accounts.group.load()?;
    let group_pk = &ctx.accounts.group.key();

    let mut liqor = ctx.accounts.liqor.load_mut()?;
    // account constraint #1
    require!(
        liqor
            .fixed
            .is_owner_or_delegate(ctx.accounts.liqor_owner.key()),
        MangoError::SomeError
    );
    require!(!liqor.fixed.being_liquidated(), MangoError::BeingLiquidated);

    let mut liqee = ctx.accounts.liqee.load_mut()?;
    let mut liqee_health_cache = {
        let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)?;
        new_health_cache(&liqee.borrow(), &account_retriever)
            .context("create liqee health cache")?
    };

    // Check if liqee is bankrupt
    require!(
        !liqee_health_cache.has_liquidatable_assets(),
        MangoError::IsNotBankrupt
    );
    liqee.fixed.set_being_liquidated(true);

    // Find bankrupt liab amount
    let mut perp_market = ctx.accounts.perp_market.load_mut()?;
    let liqee_perp_position = liqee.perp_position_mut(perp_market.perp_market_index)?;
    require_msg!(
        liqee_perp_position.base_position_lots() == 0,
        "liqee must have zero base position"
    );
    require!(
        !liqee_perp_position.has_open_orders(),
        MangoError::HasOpenPerpOrders
    );

    let liqee_pnl = liqee_perp_position.quote_position_native();
    require_msg!(
        liqee_pnl.is_negative(),
        "liqee pnl must be negative, was {}",
        liqee_pnl
    );
    let liab_transfer = (-liqee_pnl).min(I80F48::from(max_liab_transfer));

    // Preparation for covering it with the insurance fund
    let insurance_vault_amount = if perp_market.elligible_for_group_insurance_fund() {
        ctx.accounts.insurance_vault.amount
    } else {
        0
    };

    let liquidation_fee_factor = cm!(I80F48::ONE + perp_market.liquidation_fee);

    let insurance_transfer = cm!(liab_transfer * liquidation_fee_factor)
        .checked_ceil()
        .unwrap()
        .checked_to_num::<u64>()
        .unwrap()
        .min(insurance_vault_amount);

    let insurance_transfer_i80f48 = I80F48::from(insurance_transfer);
    let insurance_fund_exhausted = insurance_transfer == insurance_vault_amount;
    let insurance_liab_transfer =
        cm!(insurance_transfer_i80f48 / liquidation_fee_factor).min(liab_transfer);

    // Try using the insurance fund if possible
    if insurance_transfer > 0 {
        let mut quote_bank = ctx.accounts.quote_bank.load_mut()?;
        require_eq!(quote_bank.token_index, QUOTE_TOKEN_INDEX);
        require_keys_eq!(quote_bank.mint, ctx.accounts.insurance_vault.mint);

        // move insurance assets into quote bank
        let group_seeds = group_seeds!(group);
        token::transfer(
            ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
            insurance_transfer,
        )?;

        // credit the liqor with quote tokens
        let (liqor_quote, _, _) = liqor.ensure_token_position(QUOTE_TOKEN_INDEX)?;
        quote_bank.deposit(liqor_quote, insurance_transfer_i80f48)?;

        // transfer perp quote loss from the liqee to the liqor
        let liqor_perp_position = liqor
            .ensure_perp_position(perp_market.perp_market_index, QUOTE_TOKEN_INDEX)?
            .0;
        liqee_perp_position.change_quote_position(insurance_liab_transfer);
        liqor_perp_position.change_quote_position(-insurance_liab_transfer);
    }

    // Socialize loss if the insurance fund is exhausted
    let remaining_liab = liab_transfer - insurance_liab_transfer;
    let mut socialized_loss = I80F48::ZERO;
    if insurance_fund_exhausted && remaining_liab.is_positive() {
        perp_market.socialize_loss(-remaining_liab)?;
        liqee_perp_position.change_quote_position(remaining_liab);
        require_eq!(liqee_perp_position.quote_position_native(), 0);
        socialized_loss = remaining_liab;
    }

    // Check liqee health again
    liqee_health_cache.recompute_perp_info(liqee_perp_position, &perp_market)?;
    let liqee_init_health = liqee_health_cache.health(HealthType::Init);
    liqee
        .fixed
        .maybe_recover_from_being_liquidated(liqee_init_health);

    emit!(PerpLiqBankruptcyLog {
        mango_group: ctx.accounts.group.key(),
        liqee: ctx.accounts.liqee.key(),
        liqor: ctx.accounts.liqor.key(),
        market_index: perp_market.perp_market_index,
        insurance_transfer: insurance_transfer_i80f48.to_bits(),
        socialized_loss: socialized_loss.to_bits()
    });

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.liqor.key(),
        perp_market.perp_market_index,
        liqor.perp_position(perp_market.perp_market_index).unwrap(),
        &perp_market,
    );

    emit_perp_balances(
        ctx.accounts.group.key(),
        ctx.accounts.liqee.key(),
        perp_market.perp_market_index,
        liqee.perp_position(perp_market.perp_market_index).unwrap(),
        &perp_market,
    );

    drop(perp_market);

    // Check liqor's health
    if !liqor.fixed.is_in_health_region() {
        let account_retriever = ScanningAccountRetriever::new(ctx.remaining_accounts, group_pk)?;
        let liqor_health = compute_health(&liqor.borrow(), HealthType::Init, &account_retriever)
            .context("compute liqor health")?;
        require!(liqor_health >= 0, MangoError::HealthMustBePositive);
    }

    Ok(())
}
