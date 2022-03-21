use super::{book::Book, metadata::MetaData, orders::Side};
use crate::error::MangoError;
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use mango_macro::Pod;

/// This will hold top level info about the perps market
/// Likely all perps transactions on a market will be locked on this one because this will be passed in as writable
#[account(zero_copy)]
pub struct PerpMarket {
    pub meta_data: MetaData,

    pub mango_group: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_queue: Pubkey,
    pub quote_lot_size: i64, // number of quote native that reresents min tick
    pub base_lot_size: i64,  // represents number of base native quantity; greater than 0

    // TODO - consider just moving this into the cache
    pub long_funding: I80F48,
    pub short_funding: I80F48,

    pub open_interest: i64, // This is i64 to keep consistent with the units of contracts, but should always be > 0

    pub last_updated: u64,
    pub seq_num: u64,
    pub fees_accrued: I80F48, // native quote currency

    pub liquidity_mining_info: LiquidityMiningInfo,

    // mngo_vault holds mango tokens to be disbursed as liquidity incentives for this perp market
    pub mngo_vault: Pubkey,
}

impl PerpMarket {
    // pub fn load_and_init<'a>(
    //     account: &'a AccountInfo,
    //     program_id: &Pubkey,
    //     mango_group_ai: &'a AccountInfo,
    //     bids_ai: &'a AccountInfo,
    //     asks_ai: &'a AccountInfo,
    //     event_queue_ai: &'a AccountInfo,
    //     mngo_vault_ai: &'a AccountInfo,
    //     mango_group: &MangoGroup,
    //     rent: &Rent,
    //     base_lot_size: i64,
    //     quote_lot_size: i64,
    //     rate: I80F48,
    //     max_depth_bps: I80F48,
    //     target_period_length: u64,
    //     mngo_per_period: u64,
    //     exp: u8,
    //     version: u8,
    //     lm_size_shift: u8, // right shift the depth number to prevent overflow
    // ) -> Result<RefMut<'a, Self>> {
    //     let mut state = Self::load_mut(account)?;
    //     check!(account.owner == program_id, MangoErrorCode::InvalidOwner)?;
    //     check!(
    //         rent.is_exempt(account.lamports(), size_of::<Self>()),
    //         MangoErrorCode::AccountNotRentExempt
    //     )?;
    //     check!(!state.meta_data.is_initialized, MangoErrorCode::Default)?;

    //     state.meta_data = MetaData::new_with_extra(
    //         DataType::PerpMarket,
    //         version,
    //         true,
    //         [exp, lm_size_shift, 0, 0, 0],
    //     );
    //     state.mango_group = *mango_group_ai.key;
    //     state.bids = *bids_ai.key;
    //     state.asks = *asks_ai.key;
    //     state.event_queue = *event_queue_ai.key;
    //     state.quote_lot_size = quote_lot_size;
    //     state.base_lot_size = base_lot_size;

    //     let vault = Account::unpack(&mngo_vault_ai.try_borrow_data()?)?;
    //     check!(
    //         vault.owner == mango_group.signer_key,
    //         MangoErrorCode::InvalidOwner
    //     )?;
    //     check!(vault.delegate.is_none(), MangoErrorCode::InvalidVault)?;
    //     check!(
    //         vault.close_authority.is_none(),
    //         MangoErrorCode::InvalidVault
    //     )?;
    //     check!(vault.mint == mngo_token::ID, MangoErrorCode::InvalidVault)?;
    //     check!(
    //         mngo_vault_ai.owner == &spl_token::ID,
    //         MangoErrorCode::InvalidOwner
    //     )?;
    //     state.mngo_vault = *mngo_vault_ai.key;

    //     let clock = Clock::get()?;
    //     let period_start = clock.unix_timestamp as u64;
    //     state.last_updated = period_start;

    //     state.liquidity_mining_info = LiquidityMiningInfo {
    //         rate,
    //         max_depth_bps,
    //         period_start,
    //         target_period_length,
    //         mngo_left: mngo_per_period,
    //         mngo_per_period,
    //     };

    //     Ok(state)
    // }

    // pub fn load_checked<'a>(
    //     account: &'a AccountInfo,
    //     program_id: &Pubkey,
    //     mango_group_pk: &Pubkey,
    // ) -> MangoResult<Ref<'a, Self>> {
    //     check_eq!(account.owner, program_id, MangoErrorCode::InvalidOwner)?;
    //     let state = Self::load(account)?;
    //     check!(state.meta_data.is_initialized, MangoErrorCode::Default)?;
    //     check!(
    //         state.meta_data.data_type == DataType::PerpMarket as u8,
    //         MangoErrorCode::Default
    //     )?;
    //     check!(
    //         mango_group_pk == &state.mango_group,
    //         MangoErrorCode::Default
    //     )?;
    //     Ok(state)
    // }

    // pub fn load_mut_checked<'a>(
    //     account: &'a AccountInfo,
    //     program_id: &Pubkey,
    //     mango_group_pk: &Pubkey,
    // ) -> MangoResult<RefMut<'a, Self>> {
    //     check_eq!(account.owner, program_id, MangoErrorCode::InvalidOwner)?;
    //     let state = Self::load_mut(account)?;
    //     check!(
    //         state.meta_data.is_initialized,
    //         MangoErrorCode::InvalidAccountState
    //     )?;
    //     check!(
    //         state.meta_data.data_type == DataType::PerpMarket as u8,
    //         MangoErrorCode::InvalidAccountState
    //     )?;
    //     check!(
    //         mango_group_pk == &state.mango_group,
    //         MangoErrorCode::InvalidAccountState
    //     )?;
    //     Ok(state)
    // }

    pub fn gen_order_id(&mut self, side: Side, price: i64) -> i128 {
        self.seq_num += 1;

        let upper = (price as i128) << 64;
        match side {
            Side::Bid => upper | (!self.seq_num as i128),
            Side::Ask => upper | (self.seq_num as i128),
        }
    }

    /// Use current order book price and index price to update the instantaneous funding
    pub fn update_funding(
        &mut self,
        mango_group: &MangoGroup,
        book: &Book,
        mango_cache: &MangoCache,
        market_index: usize,
        now_ts: u64,
    ) -> Result<()> {
        // Get the index price from cache, ensure it's not outdated
        let price_cache = &mango_cache.price_cache[market_index];
        price_cache.check_valid(&mango_group, now_ts)?;

        let index_price = price_cache.price;
        // hard-coded for now because there's no convenient place to put this; also creates breaking
        // change if we make this a parameter
        const IMPACT_QUANTITY: i64 = 100;

        // Get current book price & compare it to index price
        let bid = book.get_impact_price(Side::Bid, IMPACT_QUANTITY, now_ts);
        let ask = book.get_impact_price(Side::Ask, IMPACT_QUANTITY, now_ts);

        const MAX_FUNDING: I80F48 = I80F48!(0.05);
        const MIN_FUNDING: I80F48 = I80F48!(-0.05);

        let diff = match (bid, ask) {
            (Some(bid), Some(ask)) => {
                // calculate mid-market rate
                let book_price = self.lot_to_native_price((bid + ask) / 2);
                (book_price / index_price - I80F48::ONE).clamp(MIN_FUNDING, MAX_FUNDING)
            }
            (Some(_bid), None) => MAX_FUNDING,
            (None, Some(_ask)) => MIN_FUNDING,
            (None, None) => I80F48::ZERO,
        };

        // TODO TEST consider what happens if time_factor is very small. Can funding_delta == 0 when diff != 0?
        let time_factor = I80F48::from_num(now_ts - self.last_updated) / DAY;
        let funding_delta: I80F48 = index_price
            .checked_mul(diff)
            .unwrap()
            .checked_mul(I80F48::from_num(self.base_lot_size))
            .unwrap()
            .checked_mul(time_factor)
            .unwrap();

        self.long_funding += funding_delta;
        self.short_funding += funding_delta;
        self.last_updated = now_ts;

        // Check if liquidity incentives ought to be paid out and if so pay them out
        Ok(())
    }

    /// Convert from the price stored on the book to the price used in value calculations
    pub fn lot_to_native_price(&self, price: i64) -> I80F48 {
        I80F48::from_num(price)
            .checked_mul(I80F48::from_num(self.quote_lot_size))
            .unwrap()
            .checked_div(I80F48::from_num(self.base_lot_size))
            .unwrap()
    }

    /// Socialize the loss in this account across all longs and shorts
    pub fn socialize_loss(
        &mut self,
        account: &mut PerpAccount,
        cache: &mut PerpMarketCache,
    ) -> Result<I80F48> {
        // TODO convert into only socializing on one side
        // native USDC per contract open interest
        let socialized_loss = if self.open_interest == 0 {
            // This is kind of an unfortunate situation. This means socialized loss occurs on the
            // last person to call settle_pnl on their profits. Any advice on better mechanism
            // would be appreciated. Luckily, this will be an extremely rare situation.
            I80F48::ZERO
        } else {
            account
                .quote_position
                .checked_div(I80F48::from_num(self.open_interest))
                .ok_or(MangoError::SomeError)?
        };
        account.quote_position = I80F48::ZERO;
        self.long_funding -= socialized_loss;
        self.short_funding += socialized_loss;

        cache.short_funding = self.short_funding;
        cache.long_funding = self.long_funding;
        Ok(socialized_loss)
    }
}

#[derive(Copy, Clone, Pod)]
#[repr(C)]
/// Information regarding market maker incentives for a perp market
pub struct LiquidityMiningInfo {
    /// Used to convert liquidity points to MNGO
    pub rate: I80F48,

    pub max_depth_bps: I80F48, // instead of max depth bps, this should be max num contracts

    /// start timestamp of current liquidity incentive period; gets updated when mngo_left goes to 0
    pub period_start: u64,

    /// Target time length of a period in seconds
    pub target_period_length: u64,

    /// Paper MNGO left for this period
    pub mngo_left: u64,

    /// Total amount of MNGO allocated for current period
    pub mngo_per_period: u64,
}
