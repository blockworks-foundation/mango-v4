#![allow(dead_code)]
use super::*;
use anchor_lang::prelude::AccountMeta;
use mango_client::send_tx;
use mango_v4::accounts_ix::{
    OpenbookV2PlaceOrderType, OpenbookV2SelfTradeBehavior, OpenbookV2Side,
};
use mango_v4::serum3_cpi::{OpenOrdersAmounts, OpenOrdersSlim};
use openbook_v2::state::Side;
use std::sync::Arc;

struct OpenbookV2OrderPlacer {
    solana: Arc<SolanaCookie>,
    openbook: Arc<OpenbookV2Cookie>,
    account: Pubkey,
    group: Pubkey,
    owner: TestKeypair,
    openbook_market: Pubkey,
    openbook_market_external: Pubkey,
    open_orders: Pubkey,
    next_client_order_id: u64,
}

impl OpenbookV2OrderPlacer {
    fn inc_client_order_id(&mut self) -> u64 {
        let id = self.next_client_order_id;
        self.next_client_order_id += 1;
        id
    }

    async fn find_order_id_for_client_order_id(&self, client_order_id: u64) -> Option<(u128, u64)> {
        println!("finding {}", client_order_id);
        let open_orders = self.openbook.load_open_orders(self.open_orders).await;
        println!(
            "in {:?}",
            open_orders
                .all_orders_in_use()
                .map(|o| o.client_id)
                .collect_vec()
        );
        match open_orders.find_order_with_client_order_id(client_order_id) {
            Some(order) => return Some((order.id, client_order_id)),
            None => return None,
        }
    }

    async fn try_bid(
        &mut self,
        limit_price: f64,
        max_base: i64,
        taker: bool,
    ) -> Result<mango_v4::accounts::OpenbookV2PlaceOrder, TransportError> {
        let client_order_id = self.inc_client_order_id();
        let fees = if taker { 0.0004 } else { 0.0 };
        send_tx(
            &self.solana,
            OpenbookV2PlaceOrderInstruction {
                side: OpenbookV2Side::Bid,
                price_lots: (limit_price * 100.0 / 10.0) as i64, // in quote_lot (10) per base lot (100)
                max_base_lots: max_base / 100,                   // in base lot (100)
                // 4 bps taker fees added in
                max_quote_lots_including_fees: (limit_price * (max_base as f64) * (1.0 + fees)
                    / 10.0)
                    .ceil() as i64,
                client_order_id,
                limit: 10,
                account: self.account,
                expiry_timestamp: 0,
                owner: self.owner,
                openbook_v2_market: self.openbook_market,
                reduce_only: false,
                order_type: OpenbookV2PlaceOrderType::Limit,
                self_trade_behavior: OpenbookV2SelfTradeBehavior::AbortTransaction,
            },
        )
        .await
    }

    async fn bid_maker(&mut self, limit_price: f64, max_base: i64) -> Option<(u128, u64)> {
        self.try_bid(limit_price, max_base, false).await.unwrap();
        self.find_order_id_for_client_order_id(self.next_client_order_id - 1)
            .await
    }

    async fn bid_taker(&mut self, limit_price: f64, max_base: i64) {
        self.try_bid(limit_price, max_base, true).await.unwrap();
    }

    async fn try_ask(
        &mut self,
        limit_price: f64,
        max_base: i64,
        taker: bool,
    ) -> Result<mango_v4::accounts::OpenbookV2PlaceOrder, TransportError> {
        let client_order_id = self.inc_client_order_id();
        let fees = if taker { 0.0004 } else { 0.0 };
        send_tx(
            &self.solana,
            OpenbookV2PlaceOrderInstruction {
                side: OpenbookV2Side::Ask,
                price_lots: (limit_price * 100.0 / 10.0) as i64, // in quote_lot (10) per base lot (100)
                max_base_lots: max_base / 100,                   // in base lot (100)
                max_quote_lots_including_fees: (limit_price * (max_base as f64) * (1.0 + fees)
                    / 10.0)
                    .ceil() as i64,
                client_order_id,
                limit: 10,
                account: self.account,
                expiry_timestamp: 0,
                owner: self.owner,
                openbook_v2_market: self.openbook_market,
                reduce_only: false,
                order_type: OpenbookV2PlaceOrderType::Limit,
                self_trade_behavior: OpenbookV2SelfTradeBehavior::AbortTransaction,
            },
        )
        .await
    }

    async fn ask_taker(&mut self, limit_price: f64, max_base: i64) {
        self.try_ask(limit_price, max_base, true).await.unwrap();
    }

    async fn ask_maker(&mut self, limit_price: f64, max_base: i64) -> Option<(u128, u64)> {
        self.try_ask(limit_price, max_base, false).await.unwrap();
        self.find_order_id_for_client_order_id(self.next_client_order_id - 1)
            .await
    }

    async fn cancel(&self, order_id: u128) {
        let side = {
            let open_orders = self.openbook.load_open_orders(self.open_orders).await;
            open_orders
                .find_order_with_order_id(order_id)
                .unwrap()
                .side_and_tree()
                .side()
        };
        send_tx(
            &self.solana,
            OpenbookV2CancelOrderInstruction {
                side: match side {
                    Side::Ask => OpenbookV2Side::Ask,
                    Side::Bid => OpenbookV2Side::Bid,
                },
                order_id,
                account: self.account,
                payer: self.owner,
                openbook_v2_market: self.openbook_market,
            },
        )
        .await
        .unwrap();
    }

    async fn cancel_all(&self) {
        send_tx(
            &self.solana,
            OpenbookV2CancelAllOrdersInstruction {
                side_opt: None,
                limit: 16,
                account: self.account,
                payer: self.owner,
                openbook_v2_market: self.openbook_market,
            },
        )
        .await
        .unwrap();
    }

    async fn settle(&self, fees_to_dao: bool) {
        send_tx(
            &self.solana,
            OpenbookV2SettleFundsInstruction {
                owner: self.owner,
                account: self.account,
                openbook_v2_market: self.openbook_market,
                fees_to_dao: fees_to_dao,
            },
        )
        .await
        .unwrap();
    }

    async fn mango_openbook_orders(&self) -> OpenbookV2Orders {
        let account_data = get_mango_account(&self.solana, self.account).await;
        let orders = account_data
            .all_openbook_v2_orders()
            .find(|s| s.open_orders == self.open_orders)
            .unwrap();
        orders.clone()
    }

    async fn open_orders(&self) -> OpenOrdersSlim {
        let data = self
            .solana
            .get_account::<openbook_v2::state::OpenOrdersAccount>(self.open_orders)
            .await;
        OpenOrdersSlim::from_oo_v2(&data, 100, 10)
    }
}

#[tokio::test]
async fn test_openbook_basics() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(200_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..2];

    //
    // SETUP: Create a group and an account
    //

    let GroupWithTokens { group, tokens, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let base_token = &tokens[0];
    let quote_token = &tokens[1];

    //
    // SETUP: Create openbook market
    //
    let openbook_market_cookie = context
        .openbook
        .list_spot_market(&quote_token.mint, &base_token.mint, payer)
        .await;

    //
    // TEST: Register a openbook market
    //
    let openbook_v2_market = send_tx(
        solana,
        OpenbookV2RegisterMarketInstruction {
            group,
            admin,
            openbook_v2_program: context.openbook.program_id,
            openbook_v2_market_external: openbook_market_cookie.market,
            market_index: 0,
            base_bank: base_token.bank,
            quote_bank: quote_token.bank,
            payer,
        },
    )
    .await
    .unwrap()
    .openbook_v2_market;

    //
    // SETUP: Create account
    //
    let deposit_amount = 1000;
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;

    //
    // TEST: Create an open orders account
    //
    let open_orders_account = send_tx(
        solana,
        OpenbookV2CreateOpenOrdersInstruction {
            group,
            payer,
            owner,
            account,
            openbook_v2_market,
            openbook_v2_program: context.openbook.program_id,
            openbook_v2_market_external: openbook_market_cookie.market,
            next_open_orders_index: 1,
        },
    )
    .await
    .unwrap()
    .open_orders_account;

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data
            .active_openbook_v2_orders()
            .map(|v| (v.open_orders, v.market_index))
            .collect::<Vec<_>>(),
        [(open_orders_account, 0)]
    );

    let mut order_placer = OpenbookV2OrderPlacer {
        solana: solana.clone(),
        openbook: context.openbook.clone(),
        account,
        group,
        owner,
        openbook_market: openbook_v2_market,
        openbook_market_external: openbook_market_cookie.market,
        open_orders: open_orders_account,
        next_client_order_id: 0,
    };

    //
    // TEST: Place an order
    //
    let (order_id, _) = order_placer.bid_maker(0.9, 100).await.unwrap();
    check_prev_instruction_post_health(&solana, account).await;

    let native0 = account_position(solana, account, base_token.bank).await;
    let native1 = account_position(solana, account, quote_token.bank).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 910);

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data
            .token_position_by_raw_index(0)
            .unwrap()
            .in_use_count,
        1
    );
    assert_eq!(
        account_data
            .token_position_by_raw_index(1)
            .unwrap()
            .in_use_count,
        1
    );
    assert_eq!(
        account_data
            .token_position_by_raw_index(2)
            .unwrap()
            .in_use_count,
        0
    );
    let openbook_orders = account_data.openbook_v2_orders_by_raw_index(0).unwrap();
    assert_eq!(openbook_orders.base_borrows_without_fee, 0);
    assert_eq!(openbook_orders.quote_borrows_without_fee, 0);
    assert_eq!(openbook_orders.potential_base_tokens, 100);
    assert_eq!(openbook_orders.potential_quote_tokens, 90);

    let base_bank = solana.get_account::<Bank>(base_token.bank).await;
    assert_eq!(base_bank.potential_openbook_tokens, 100);
    let quote_bank = solana.get_account::<Bank>(quote_token.bank).await;
    assert_eq!(quote_bank.potential_openbook_tokens, 90);

    assert!(order_id != 0);

    //
    // TEST: Cancel the order
    //
    order_placer.cancel(order_id).await;

    //
    // TEST: Settle, moving the freed up funds back
    //
    order_placer.settle(false).await;

    let native0 = account_position(solana, account, base_token.bank).await;
    let native1 = account_position(solana, account, quote_token.bank).await;
    assert_eq!(native0, 1000);
    assert_eq!(native1, 1000);

    let account_data = get_mango_account(solana, account).await;
    let openbook_orders = account_data.openbook_v2_orders_by_raw_index(0).unwrap();
    assert_eq!(openbook_orders.base_borrows_without_fee, 0);
    assert_eq!(openbook_orders.quote_borrows_without_fee, 0);
    assert_eq!(openbook_orders.potential_base_tokens, 0);
    assert_eq!(openbook_orders.potential_quote_tokens, 0);

    let base_bank = solana.get_account::<Bank>(base_token.bank).await;
    assert_eq!(base_bank.potential_openbook_tokens, 0);
    let quote_bank = solana.get_account::<Bank>(quote_token.bank).await;
    assert_eq!(quote_bank.potential_openbook_tokens, 0);

    // Process events such that the OutEvent deactivates the closed order on open_orders
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;

    // close oo account
    send_tx(
        solana,
        OpenbookV2CloseOpenOrdersInstruction {
            account,
            openbook_v2_market,
            owner,
            sol_destination: owner.pubkey(),
        },
    )
    .await
    .unwrap();

    let account_data = get_mango_account(solana, account).await;
    assert_eq!(
        account_data
            .token_position_by_raw_index(0)
            .unwrap()
            .in_use_count,
        0
    );
    assert_eq!(
        account_data
            .token_position_by_raw_index(1)
            .unwrap()
            .in_use_count,
        0
    );

    // deregister market
    send_tx(
        solana,
        OpenbookV2DeregisterMarketInstruction {
            group,
            admin,
            payer,
            openbook_v2_market_external: openbook_market_cookie.market,
            market_index: 0,
        },
    )
    .await
    .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_openbook_loan_origination_fees() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 180000;
    let CommonSetup {
        openbook_market_cookie,
        quote_token,
        base_token,
        mut order_placer,
        mut order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;
    let quote_bank = quote_token.bank;
    let base_bank = base_token.bank;
    let account = order_placer.account;
    let account2 = order_placer2.account;

    //
    // TEST: Placing and canceling an order does not take loan origination fees even if borrows are needed
    //
    {
        let (bid_order_id, _) = order_placer.bid_maker(1.0, 200000).await.unwrap();
        let (ask_order_id, _) = order_placer.ask_maker(2.0, 200000).await.unwrap();

        let o = order_placer.mango_openbook_orders().await;
        assert_eq!(o.base_borrows_without_fee, 19999); // rounded
        assert_eq!(o.quote_borrows_without_fee, 19999);

        order_placer.cancel(bid_order_id).await;
        order_placer.cancel(ask_order_id).await;

        let o = order_placer.mango_openbook_orders().await;
        assert_eq!(o.base_borrows_without_fee, 19999); // unchanged
        assert_eq!(o.quote_borrows_without_fee, 19999);

        // placing new, slightly larger orders increases the borrow_without_fee amount only by a small amount
        let (bid_order_id, _) = order_placer.bid_maker(1.0, 210000).await.unwrap();
        let (ask_order_id, _) = order_placer.ask_maker(2.0, 300000).await.unwrap();

        let o = order_placer.mango_openbook_orders().await;
        assert_eq!(o.base_borrows_without_fee, 119998); // rounded
        assert_eq!(o.quote_borrows_without_fee, 29998);

        order_placer.cancel(bid_order_id).await;
        order_placer.cancel(ask_order_id).await;

        // returns all the funds
        order_placer.settle(false).await;

        let o = order_placer.mango_openbook_orders().await;
        assert_eq!(o.base_borrows_without_fee, 0);
        assert_eq!(o.quote_borrows_without_fee, 0);

        assert_eq!(
            account_position(solana, account, quote_bank).await,
            deposit_amount as i64
        );
        assert_eq!(
            account_position(solana, account, base_bank).await,
            deposit_amount as i64
        );

        // consume all the out events from the cancels
        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
    }

    let without_openbook_taker_fee = |amount: i64| (amount as f64 * (1.0 - 0.0004)).trunc() as i64;
    let openbook_maker_rebate = |amount: i64| (amount as f64 * 0.0002).floor() as i64;
    let loan_origination_fee = |amount: i64| (amount as f64 * 0.0005).trunc() as i64;

    //
    // TEST: Order execution and settling charges borrow fee
    //
    {
        let deposit_amount = deposit_amount as i64;
        let bid_amount = 200000;
        let ask_amount = 210000;
        let fill_amount = 200000;
        let book_amount = ask_amount - fill_amount;
        let quote_fees1 = solana
            .get_account::<Bank>(quote_bank)
            .await
            .collected_fees_native;

        assert_eq!(
            account_position(solana, account, quote_bank).await,
            deposit_amount
        );

        // account2 has an order on the book
        order_placer2.bid_maker(1.0, bid_amount).await.unwrap();

        // account takes
        order_placer.ask_taker(1.0, ask_amount).await;
        order_placer.settle(true).await;

        let o = order_placer.mango_openbook_orders().await;
        // parts of the order ended up on the book and may cause loan origination fees later
        assert_eq!(o.base_borrows_without_fee, book_amount as u64);
        assert_eq!(o.quote_borrows_without_fee, 0);

        assert_eq!(
            account_position(solana, account, quote_bank).await,
            deposit_amount + without_openbook_taker_fee(fill_amount)
        );
        assert_eq!(
            account_position(solana, account, base_bank).await,
            deposit_amount - ask_amount - loan_origination_fee(fill_amount - deposit_amount)
        );

        // openbook referrer rebates (taker fees) accrued to the dao
        let quote_fees2 = solana
            .get_account::<Bank>(quote_bank)
            .await
            .collected_fees_native;
        assert_eq_fixed_f64!(quote_fees2 - quote_fees1, 40.0, 0.1);

        // check account2 balances too
        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
        order_placer2.settle(false).await;

        let o = order_placer2.mango_openbook_orders().await;
        assert_eq!(o.base_borrows_without_fee, 0);
        assert_eq!(o.quote_borrows_without_fee, 0);

        assert_eq!(
            account_position(solana, account2, base_bank).await,
            deposit_amount + fill_amount
        );
        assert_eq!(
            account_position(solana, account2, quote_bank).await,
            deposit_amount - fill_amount - loan_origination_fee(fill_amount - deposit_amount)
                + openbook_maker_rebate(fill_amount)
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_openbook_settle_to_dao() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 160000;
    let CommonSetup {
        group_with_tokens,
        openbook_market_cookie,
        quote_token,
        base_token,
        mut order_placer,
        mut order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;
    let quote_bank = quote_token.bank;
    let base_bank = base_token.bank;
    let account = order_placer.account;
    let account2 = order_placer2.account;

    // Change the quote price to verify that the current value of the openbook quote token
    // is added to the buyback fees amount
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &quote_token,
        group_with_tokens.admin,
        2.0,
    )
    .await;

    let openbook_taker_fee = |amount: i64| (amount as f64 * 0.0004).trunc() as i64;
    let openbook_maker_rebate = |amount: i64| (amount as f64 * 0.0002).floor() as i64;
    let openbook_referrer_fee = |amount: i64| (amount as f64 * 0.0002).trunc() as i64;
    let loan_origination_fee = |amount: i64| (amount as f64 * 0.0005).trunc() as i64;

    //
    // TEST: Use openbook_v2_settle_funds
    //
    let deposit_amount = deposit_amount as i64;
    let amount = 200000;
    let quote_fees_start = solana
        .get_account::<Bank>(quote_bank)
        .await
        .collected_fees_native;
    let quote_start = account_position(solana, account, quote_bank).await;
    let quote2_start = account_position(solana, account2, quote_bank).await;
    let base_start = account_position(solana, account, base_bank).await;
    let base2_start = account_position(solana, account2, base_bank).await;

    // account2 has an order on the book, account takes
    order_placer2.bid_maker(1.0, amount).await.unwrap();
    order_placer.ask_taker(1.0, amount).await;

    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;

    order_placer.settle(true).await;
    order_placer2.settle(true).await;

    let quote_end = account_position(solana, account, quote_bank).await;
    let quote2_end = account_position(solana, account2, quote_bank).await;
    let base_end = account_position(solana, account, base_bank).await;
    let base2_end = account_position(solana, account2, base_bank).await;

    let lof = loan_origination_fee(amount - deposit_amount);
    assert_eq!(base_start - amount - lof, base_end);
    assert_eq!(base2_start + amount, base2_end);
    assert_eq!(quote_start + amount - openbook_taker_fee(amount), quote_end);
    assert_eq!(
        quote2_start - amount + openbook_maker_rebate(amount) - lof,
        quote2_end
    );

    let quote_fees_end = solana
        .get_account::<Bank>(quote_bank)
        .await
        .collected_fees_native;
    assert_eq_fixed_f64!(
        quote_fees_end - quote_fees_start,
        (lof + openbook_referrer_fee(amount)) as f64,
        0.1
    );

    let account_data = solana.get_account::<MangoAccount>(account).await;
    assert_eq!(
        account_data.buyback_fees_accrued_current,
        (openbook_maker_rebate(amount) * 2) as u64 // *2 because that's the quote price and this number is in $
    );
    let account2_data = solana.get_account::<MangoAccount>(account2).await;
    assert_eq!(account2_data.buyback_fees_accrued_current, 0);

    Ok(())
}

#[tokio::test]
async fn test_openbook_settle_to_account() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 160000;
    let CommonSetup {
        openbook_market_cookie,
        quote_token,
        base_token,
        mut order_placer,
        mut order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;
    let quote_bank = quote_token.bank;
    let base_bank = base_token.bank;
    let account = order_placer.account;
    let account2 = order_placer2.account;

    let openbook_taker_fee = |amount: i64| (amount as f64 * 0.0004).trunc() as i64;
    let openbook_maker_rebate = |amount: i64| (amount as f64 * 0.0002).floor() as i64;
    let openbook_referrer_fee = |amount: i64| (amount as f64 * 0.0002).trunc() as i64;
    let loan_origination_fee = |amount: i64| (amount as f64 * 0.0005).trunc() as i64;

    //
    // TEST: Use openbook_v2_settle_funds
    //
    let deposit_amount = deposit_amount as i64;
    let amount = 200000;
    let quote_fees_start = solana
        .get_account::<Bank>(quote_bank)
        .await
        .collected_fees_native;
    let quote_start = account_position(solana, account, quote_bank).await;
    let quote2_start = account_position(solana, account2, quote_bank).await;
    let base_start = account_position(solana, account, base_bank).await;
    let base2_start = account_position(solana, account2, base_bank).await;

    // account2 has an order on the book, account takes
    order_placer2.bid_maker(1.0, amount).await.unwrap();
    order_placer.ask_taker(1.0, amount).await;

    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;

    order_placer.settle(false).await;
    order_placer2.settle(false).await;

    let quote_end = account_position(solana, account, quote_bank).await;
    let quote2_end = account_position(solana, account2, quote_bank).await;
    let base_end = account_position(solana, account, base_bank).await;
    let base2_end = account_position(solana, account2, base_bank).await;

    let lof = loan_origination_fee(amount - deposit_amount);
    assert_eq!(base_start - amount - lof, base_end);
    assert_eq!(base2_start + amount, base2_end);
    assert_eq!(
        quote_start + amount - openbook_taker_fee(amount) + openbook_referrer_fee(amount),
        quote_end
    );
    assert_eq!(
        quote2_start - amount + openbook_maker_rebate(amount) - lof,
        quote2_end
    );

    let quote_fees_end = solana
        .get_account::<Bank>(quote_bank)
        .await
        .collected_fees_native;
    assert_eq_fixed_f64!(quote_fees_end - quote_fees_start, lof as f64, 0.1);

    let account_data = solana.get_account::<MangoAccount>(account).await;
    assert_eq!(account_data.buyback_fees_accrued_current, 0);
    let account2_data = solana.get_account::<MangoAccount>(account2).await;
    assert_eq!(account2_data.buyback_fees_accrued_current, 0);

    Ok(())
}

#[tokio::test]
async fn test_openbook_reduce_only_borrows() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 1000;
    let CommonSetup {
        group_with_tokens,
        base_token,
        mut order_placer,
        ..
    } = common_setup(&context, deposit_amount).await;

    send_tx(
        solana,
        TokenMakeReduceOnly {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: base_token.mint.pubkey,
            reduce_only: 2,
            force_close: false,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Cannot borrow tokens when bank is reduce only
    //
    let err = order_placer.try_ask(1.0, 1100, true).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    order_placer.try_ask(0.5, 500, true).await.unwrap();

    let err = order_placer.try_ask(1.0, 600, true).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    order_placer.try_ask(2.0, 500, true).await.unwrap();

    let err = order_placer.try_ask(1.0, 100, true).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    Ok(())
}

#[tokio::test]
async fn test_openbook_reduce_only_deposits1() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 1000;
    let CommonSetup {
        group_with_tokens,
        base_token,
        mut order_placer,
        mut order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;

    send_tx(
        solana,
        TokenMakeReduceOnly {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: base_token.mint.pubkey,
            reduce_only: 1,
            force_close: false,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Cannot buy tokens when deposits are already >0
    //

    // fails to place on the book
    let err = order_placer.try_bid(1.0, 1000, false).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    // also fails as a taker order
    order_placer2.ask_taker(1.0, 500).await;
    let err = order_placer.try_bid(1.0, 100, true).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    Ok(())
}

#[tokio::test]
async fn test_openbook_reduce_only_deposits2() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 1000;
    let CommonSetup {
        group_with_tokens,
        base_token,
        mut order_placer,
        mut order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;

    // Give account some base token borrows (-500)
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1500,
            allow_borrow: true,
            account: order_placer.account,
            owner: order_placer.owner,
            token_account: context.users[0].token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Cannot buy tokens when deposits are already >0
    //
    send_tx(
        solana,
        TokenMakeReduceOnly {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: base_token.mint.pubkey,
            reduce_only: 1,
            force_close: false,
        },
    )
    .await
    .unwrap();

    // cannot place a large order on the book that would deposit too much
    let err = order_placer.try_bid(1.0, 600, false).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    // a small order is fine
    order_placer.try_bid(1.0, 100, false).await.unwrap();

    // taking some is fine too
    order_placer2.ask_taker(1.0, 800).await;
    order_placer.try_bid(1.0, 100, true).await.unwrap();

    // the limit for orders is reduced now, 100 received, 100 on the book
    let err = order_placer.try_bid(1.0, 400, true).await;
    assert_mango_error(&err, MangoError::TokenInReduceOnlyMode.into(), "".into());

    Ok(())
}

#[tokio::test]
async fn test_openbook_place_reducing_when_liquidatable() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 1000;
    let CommonSetup {
        group_with_tokens,
        base_token,
        mut order_placer,
        ..
    } = common_setup(&context, deposit_amount).await;

    // Give account some base token borrows (-500)
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1500,
            allow_borrow: true,
            account: order_placer.account,
            owner: order_placer.owner,
            token_account: context.users[0].token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Change the base price to make the account liquidatable
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &base_token,
        group_with_tokens.admin,
        10.0,
    )
    .await;

    assert!(account_init_health(solana, order_placer.account).await < 0.0);

    // can place an order that would close some of the borrows
    order_placer.try_bid(10.0, 200, false).await.unwrap();

    // if too much base is bought, health would decrease: forbidden
    let err = order_placer.try_bid(10.0, 800, false).await;
    assert_mango_error(
        &err,
        MangoError::HealthMustBePositiveOrIncrease.into(),
        "".into(),
    );

    Ok(())
}

#[tokio::test]
async fn test_openbook_liq_force_close() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 1000;
    let CommonSetup {
        group_with_tokens,
        base_token,
        mut order_placer,
        ..
    } = common_setup(&context, deposit_amount).await;

    // Place orders that can later be cancelled
    order_placer.try_bid(0.8, 200, false).await.unwrap();
    order_placer.try_ask(1.2, 200, false).await.unwrap();

    // Give account some base token borrows (-500)
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1500,
            allow_borrow: true,
            account: order_placer.account,
            owner: order_placer.owner,
            token_account: context.users[0].token_accounts[1],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Change the base price to make the account liquidatable
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &base_token,
        group_with_tokens.admin,
        10.0,
    )
    .await;

    let before_init_health = account_init_health(solana, order_placer.account).await;
    assert!(before_init_health < 0.0);

    send_tx(
        solana,
        OpenbookV2LiqForceCancelInstruction {
            account: order_placer.account,
            payer: context.users[1].key,
            openbook_v2_market: order_placer.openbook_market,
        },
    )
    .await
    .unwrap();

    let after_init_health = account_init_health(solana, order_placer.account).await;
    assert!(after_init_health > before_init_health);

    let oo = order_placer.open_orders().await;
    assert_eq!(oo.native_quote_reserved(), 0);
    assert_eq!(oo.native_base_reserved(), 0);

    Ok(())
}

#[tokio::test]
async fn test_openbook_track_bid_ask() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 10000;
    let CommonSetup {
        openbook_market_cookie,
        mut order_placer,
        ..
    } = common_setup(&context, deposit_amount).await;

    //
    // TEST: highest bid/lowest ask updating
    //

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 0.0);
    assert_eq!(srm.lowest_placed_bid_inv, 0.0);
    assert_eq!(srm.highest_placed_ask, 0.0);
    assert_eq!(srm.lowest_placed_ask, 0.0);

    order_placer.bid_maker(10.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 10.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 10.0);
    assert_eq!(srm.highest_placed_ask, 0.0);
    assert_eq!(srm.lowest_placed_ask, 0.0);

    order_placer.bid_maker(9.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 10.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 9.0);
    assert_eq!(srm.highest_placed_ask, 0.0);
    assert_eq!(srm.lowest_placed_ask, 0.0);

    order_placer.bid_maker(11.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 11.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 9.0);
    assert_eq!(srm.highest_placed_ask, 0.0);
    assert_eq!(srm.lowest_placed_ask, 0.0);

    order_placer.ask_maker(20.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 11.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 9.0);
    assert_eq!(srm.highest_placed_ask, 20.0);
    assert_eq!(srm.lowest_placed_ask, 20.0);

    order_placer.ask_maker(19.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 11.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 9.0);
    assert_eq!(srm.highest_placed_ask, 20.0);
    assert_eq!(srm.lowest_placed_ask, 19.0);

    order_placer.ask_maker(21.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 11.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 9.0);
    assert_eq!(srm.highest_placed_ask, 21.0);
    assert_eq!(srm.lowest_placed_ask, 19.0);

    //
    // TEST: cancellation allows for resets
    //

    order_placer.cancel_all().await;

    // no immediate change
    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0 / 11.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0 / 9.0);
    assert_eq!(srm.highest_placed_ask, 21.0);
    assert_eq!(srm.lowest_placed_ask, 19.0);

    // Process events such that the OutEvent deactivates the closed order on open_orders
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;

    // takes new value for bid, resets ask
    order_placer.bid_maker(1.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0);
    assert_eq!(srm.highest_placed_ask, 0.0);
    assert_eq!(srm.lowest_placed_ask, 0.0);

    //
    // TEST: can reset even when there's still an order on the other side
    //
    let (oid, _) = order_placer.ask_maker(10.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0);
    assert_eq!(srm.highest_placed_ask, 10.0);
    assert_eq!(srm.lowest_placed_ask, 10.0);

    order_placer.cancel(oid).await;
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;
    order_placer.ask_maker(9.0, 100).await.unwrap();

    let srm = order_placer.mango_openbook_orders().await;
    assert_eq!(srm.highest_placed_bid_inv, 1.0);
    assert_eq!(srm.lowest_placed_bid_inv, 1.0);
    assert_eq!(srm.highest_placed_ask, 9.0);
    assert_eq!(srm.lowest_placed_ask, 9.0);

    Ok(())
}

#[tokio::test]
async fn test_openbook_track_reserved_deposits() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 100000;
    let CommonSetup {
        openbook_market_cookie,
        quote_token,
        base_token,
        mut order_placer,
        mut order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;
    let solana = &context.solana.clone();
    let quote_bank = quote_token.bank;
    let base_bank = base_token.bank;
    let account = order_placer.account;

    let get_vals = |solana| async move {
        let account_data = get_mango_account(solana, account).await;
        let orders = account_data.all_openbook_v2_orders().next().unwrap();
        let base_bank = solana.get_account::<Bank>(base_bank).await;
        let quote_bank = solana.get_account::<Bank>(quote_bank).await;
        (
            orders.potential_base_tokens,
            base_bank.potential_openbook_tokens,
            orders.potential_quote_tokens,
            quote_bank.potential_openbook_tokens,
        )
    };

    //
    // TEST: place a bid and ask and observe tracking
    //

    order_placer.bid_maker(0.8, 2000).await.unwrap();
    assert_eq!(get_vals(solana).await, (2000, 2000, 1600, 1600));

    order_placer.ask_maker(1.2, 2000).await.unwrap();
    assert_eq!(
        get_vals(solana).await,
        (2 * 2000, 2 * 2000, 1600 + 2400, 1600 + 2400)
    );

    //
    // TEST: match partially on both sides, increasing the on-bank reserved amounts
    // because order_placer2 puts funds into the openbook oo
    //

    order_placer2.bid_taker(1.2, 1000).await;
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;
    // taker order directly converted to base, no change to quote
    assert_eq!(get_vals(solana).await, (4000, 4000 + 1000, 4000, 4000));

    // takes out 1000 base
    order_placer2.settle(false).await;
    assert_eq!(get_vals(solana).await, (4000, 4000, 4000, 4000));

    order_placer2.ask_taker(0.8, 1000).await;
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;
    // taker order directly converted to quote
    assert_eq!(get_vals(solana).await, (4000, 4000, 4000, 4000 + 799));

    order_placer2.settle(false).await;
    assert_eq!(get_vals(solana).await, (4000, 4000, 4000, 4000));

    //
    // TEST: Settlement updates the values
    //

    order_placer.settle(false).await;
    // remaining is bid 1000 @ 0.8; ask 1000 @ 1.2
    assert_eq!(get_vals(solana).await, (2000, 2000, 2000, 2000));

    Ok(())
}

#[tokio::test]
async fn test_openbook_compute() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000); // place order needs lots
    let context = test_builder.start_default().await;
    let solana = &context.solana;

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 100000;
    let CommonSetup {
        openbook_market_cookie,
        mut order_placer,
        order_placer2,
        ..
    } = common_setup(&context, deposit_amount).await;

    //
    // TEST: check compute per openbook match
    //

    for limit in 1..6 {
        order_placer.bid_maker(1.0, 100).await.unwrap();
        order_placer.bid_maker(1.1, 100).await.unwrap();
        order_placer.bid_maker(1.2, 100).await.unwrap();
        order_placer.bid_maker(1.3, 100).await.unwrap();
        order_placer.bid_maker(1.4, 100).await.unwrap();

        let result = send_tx_get_metadata(
            solana,
            OpenbookV2PlaceOrderInstruction {
                side: OpenbookV2Side::Ask,
                price_lots: (1.0 * 100.0 / 10.0) as i64, // in quote_lot (10) per base lot (100)
                max_base_lots: 500 / 100,                // in base lot (100)
                max_quote_lots_including_fees: (1.0 * (500 as f64) / 10.0).ceil() as i64,
                client_order_id: 0,
                limit,
                account: order_placer2.account,
                expiry_timestamp: u64::MAX,
                owner: order_placer2.owner,
                openbook_v2_market: order_placer2.openbook_market,
                reduce_only: false,
                order_type: OpenbookV2PlaceOrderType::Limit,
                self_trade_behavior: OpenbookV2SelfTradeBehavior::AbortTransaction,
            },
        )
        .await
        .unwrap();
        println!(
            "CU for openbook_V2_place_order matching {limit} orders in sequence: {}",
            result.metadata.unwrap().compute_units_consumed
        );

        // many events need processing
        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
        order_placer.cancel_all().await;
        order_placer2.cancel_all().await;
        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
    }

    //
    // TEST: check compute per openbook cancel
    //

    for limit in 1..6 {
        for i in 0..limit {
            order_placer.bid_maker(1.0 + i as f64, 100).await.unwrap();
        }

        let result = send_tx_get_metadata(
            solana,
            OpenbookV2CancelAllOrdersInstruction {
                side_opt: None,
                limit: 10,
                account: order_placer.account,
                payer: order_placer.owner,
                openbook_v2_market: order_placer.openbook_market,
            },
        )
        .await
        .unwrap();
        println!(
            "CU for openbook_v2_cancel_all_order for {limit} orders: {}",
            result.metadata.unwrap().compute_units_consumed
        );

        context
            .openbook
            .consume_spot_events(&openbook_market_cookie, 16)
            .await;
    }

    Ok(())
}

#[tokio::test]
async fn test_fallback_oracle_openbook() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    let fallback_oracle_kp = TestKeypair::new();
    let fallback_oracle = fallback_oracle_kp.pubkey();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let payer_token_accounts = &context.users[1].token_accounts[0..3];

    //
    // SETUP: Create a group and an account
    //
    let deposit_amount = 1_000;
    let CommonSetup {
        group_with_tokens,
        quote_token,
        base_token,
        mut order_placer,
        ..
    } = common_setup(&context, deposit_amount).await;
    let GroupWithTokens {
        group,
        admin,
        tokens,
        ..
    } = group_with_tokens;

    //
    // SETUP: Create a fallback oracle
    //
    send_tx(
        solana,
        StubOracleCreate {
            oracle: fallback_oracle_kp,
            group,
            mint: tokens[2].mint.pubkey,
            admin,
            payer,
        },
    )
    .await
    .unwrap();

    //
    // SETUP: Add a fallback oracle
    //
    send_tx(
        solana,
        TokenEdit {
            group,
            admin,
            mint: tokens[2].mint.pubkey,
            fallback_oracle,
            options: mango_v4::instruction::TokenEdit {
                set_fallback_oracle: true,
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    let bank_data: Bank = solana.get_account(tokens[2].bank).await;
    assert!(bank_data.fallback_oracle == fallback_oracle);

    // Create some token1 borrows
    send_tx(
        solana,
        TokenWithdrawInstruction {
            amount: 1_500,
            allow_borrow: true,
            account: order_placer.account,
            owner,
            token_account: payer_token_accounts[2],
            bank_index: 0,
        },
    )
    .await
    .unwrap();

    // Make oracle invalid by increasing deviation
    send_tx(
        solana,
        StubOracleSetTestInstruction {
            oracle: tokens[2].oracle,
            group,
            mint: tokens[2].mint.pubkey,
            admin,
            price: 1.0,
            last_update_slot: 0,
            deviation: 100.0,
        },
    )
    .await
    .unwrap();

    //
    // TEST: Place a failing order
    //
    let limit_price = 1.0;
    let max_base = 100;
    let order_fut = order_placer.try_bid(limit_price, max_base, false).await;
    assert_mango_error(
        &order_fut,
        6023,
        "an oracle does not reach the confidence threshold".to_string(),
    );

    // now send txn with a fallback oracle in the remaining accounts
    let fallback_oracle_meta = AccountMeta {
        pubkey: fallback_oracle,
        is_writable: false,
        is_signer: false,
    };

    let client_order_id = order_placer.inc_client_order_id();
    let place_ix = OpenbookV2PlaceOrderInstruction {
        side: OpenbookV2Side::Bid,
        price_lots: (limit_price * 100.0 / 10.0) as i64, // in quote_lot (10) per base lot (100)
        max_base_lots: max_base / 100,                   // in base lot (100)
        // 4 bps taker fees added in
        max_quote_lots_including_fees: (limit_price * (max_base as f64) * (1.0) / 10.0).ceil()
            as i64,
        client_order_id,
        limit: 10,
        account: order_placer.account,
        expiry_timestamp: u64::MAX,
        owner: order_placer.owner,
        openbook_v2_market: order_placer.openbook_market,
        reduce_only: false,
        order_type: OpenbookV2PlaceOrderType::Limit,
        self_trade_behavior: OpenbookV2SelfTradeBehavior::AbortTransaction,
    };

    let result = send_tx_with_extra_accounts(solana, place_ix, vec![fallback_oracle_meta])
        .await
        .unwrap();
    result.result.unwrap();

    let account_data = get_mango_account(solana, order_placer.account).await;
    assert_eq!(
        account_data
            .token_position_by_raw_index(0)
            .unwrap()
            .in_use_count,
        1
    );
    assert_eq!(
        account_data
            .token_position_by_raw_index(1)
            .unwrap()
            .in_use_count,
        1
    );
    assert_eq!(
        account_data
            .token_position_by_raw_index(2)
            .unwrap()
            .in_use_count,
        0
    );
    let openbook_orders = account_data.openbook_v2_orders_by_raw_index(0).unwrap();
    assert_eq!(openbook_orders.base_borrows_without_fee, 0);
    assert_eq!(openbook_orders.quote_borrows_without_fee, 0);
    assert_eq!(openbook_orders.potential_base_tokens, 100);
    assert_eq!(openbook_orders.potential_quote_tokens, 100);

    let base_bank = solana.get_account::<Bank>(base_token.bank).await;
    assert_eq!(base_bank.potential_openbook_tokens, 100);
    let quote_bank = solana.get_account::<Bank>(quote_token.bank).await;
    assert_eq!(quote_bank.potential_openbook_tokens, 100);
    Ok(())
}

#[tokio::test]
async fn test_openbook_bands() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000); // place order needs lots
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 10000;
    let CommonSetup {
        group_with_tokens,
        mut order_placer,
        quote_token,
        base_token,
        ..
    } = common_setup(&context, deposit_amount).await;

    //
    // SETUP: Set oracle price for market to 100
    //
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &base_token,
        group_with_tokens.admin,
        200.0,
    )
    .await;
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &quote_token,
        group_with_tokens.admin,
        2.0,
    )
    .await;

    //
    // TEST: can place way over/under oracle
    //

    order_placer.bid_maker(1.0, 100).await.unwrap();
    order_placer.ask_maker(200.0, 100).await.unwrap();
    order_placer.cancel_all().await;

    //
    // TEST: Can't when bands are enabled
    //
    send_tx(
        solana,
        OpenbookV2EditMarketInstruction {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            market: order_placer.openbook_market,
            options: mango_v4::instruction::OpenbookV2EditMarket {
                oracle_price_band_opt: Some(0.5),
                ..openbook_v2_edit_market_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    let r = order_placer.try_bid(65.0, 100, false).await;
    assert!(r.is_err());
    let r = order_placer.try_ask(151.0, 100, false).await;
    assert!(r.is_err());

    order_placer.try_bid(67.0, 100, false).await.unwrap();
    order_placer.try_ask(149.0, 100, false).await.unwrap();

    Ok(())
}

#[tokio::test]
async fn test_openbook_deposit_limits() -> Result<(), TransportError> {
    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(150_000); // place order needs lots
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();

    //
    // SETUP: Create a group, accounts, market etc
    //
    let deposit_amount = 5000; // for 10k tokens over both order_placers
    let CommonSetup {
        openbook_market_cookie,
        group_with_tokens,
        mut order_placer,
        quote_token,
        base_token,
        ..
    } = common_setup2(&context, deposit_amount, 0).await;

    //
    // SETUP: Set oracle price for market to 2
    //
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &base_token,
        group_with_tokens.admin,
        4.0,
    )
    .await;
    set_bank_stub_oracle_price(
        solana,
        group_with_tokens.group,
        &quote_token,
        group_with_tokens.admin,
        2.0,
    )
    .await;

    //
    // SETUP: Base token: add deposit limit
    //
    send_tx(
        solana,
        TokenEdit {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: base_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                deposit_limit_opt: Some(13000),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    let solana2 = context.solana.clone();
    let base_bank = base_token.bank;
    let remaining_base = {
        || async {
            let b: Bank = solana2.get_account(base_bank).await;
            b.remaining_deposits_until_limit().round().to_num::<u64>()
        }
    };

    //
    // TEST: even when placing all base tokens into an ask, they still count
    //

    order_placer.ask_maker(2.0, 5000).await.unwrap();
    assert_eq!(remaining_base().await, 3000);

    //
    // TEST: if we bid to buy more base, the limit reduces
    //

    order_placer.bid_maker(1.5, 1000).await.unwrap();
    assert_eq!(remaining_base().await, 2000);

    //
    // TEST: if we bid too much for the limit, the order does not go through
    //

    let r = order_placer.try_bid(1.5, 2001, false).await;
    assert_mango_error(&r, MangoError::BankDepositLimit.into(), "dep limit".into());
    order_placer.try_bid(1.5, 1999, false).await.unwrap(); // not 2000 due to rounding

    //
    // SETUP: Switch deposit limit to quote token
    //

    send_tx(
        solana,
        TokenEdit {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: base_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                deposit_limit_opt: Some(0),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();
    send_tx(
        solana,
        TokenEdit {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: quote_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                deposit_limit_opt: Some(13000),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();

    let solana2 = context.solana.clone();
    let quote_bank = quote_token.bank;
    let remaining_quote = {
        || async {
            let b: Bank = solana2.get_account(quote_bank).await;
            b.remaining_deposits_until_limit().round().to_num::<i64>()
        }
    };

    order_placer.cancel_all().await;
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;

    //
    // TEST: even when placing all quote tokens into a bid, they still count
    //

    order_placer.bid_maker(2.0, 2500).await.unwrap();
    assert_eq!(remaining_quote().await, 3000);

    //
    // TEST: if we ask to get more quote, the limit reduces
    //

    order_placer.ask_maker(5.0, 200).await.unwrap();
    assert_eq!(remaining_quote().await, 2000);

    //
    // TEST: if we bid too much for the limit, the order does not go through
    //

    let r = order_placer.try_ask(5.0, 401, true).await;
    assert_mango_error(&r, MangoError::BankDepositLimit.into(), "dep limit".into());
    order_placer.try_ask(5.0, 399, true).await.unwrap(); // not 400 due to rounding

    // reset
    order_placer.cancel_all().await;
    context
        .openbook
        .consume_spot_events(&openbook_market_cookie, 16)
        .await;
    order_placer.settle(false).await;

    //
    // TEST: can place a bid even if quote deposit limit is exhausted
    //
    send_tx(
        solana,
        TokenEdit {
            group: group_with_tokens.group,
            admin: group_with_tokens.admin,
            mint: quote_token.mint.pubkey,
            fallback_oracle: Pubkey::default(),
            options: mango_v4::instruction::TokenEdit {
                deposit_limit_opt: Some(1),
                ..token_edit_instruction_default()
            },
        },
    )
    .await
    .unwrap();
    assert!(remaining_quote().await < 0);
    assert_eq!(
        account_position(solana, order_placer.account, quote_token.bank).await,
        5000
    );
    // borrowing might lead to a deposit increase later
    let r = order_placer.try_bid(1.0, 5100, false).await;
    assert_mango_error(&r, MangoError::BankDepositLimit.into(), "dep limit".into());
    // but just selling deposits is fine
    order_placer.try_bid(1.0, 4999, false).await.unwrap();

    Ok(())
}

struct CommonSetup {
    group_with_tokens: GroupWithTokens,
    openbook_market_cookie: OpenbookMarketCookie,
    quote_token: crate::program_test::mango_setup::Token,
    base_token: crate::program_test::mango_setup::Token,
    order_placer: OpenbookV2OrderPlacer,
    order_placer2: OpenbookV2OrderPlacer,
}

async fn common_setup(context: &TestContext, deposit_amount: u64) -> CommonSetup {
    common_setup2(context, deposit_amount, 10000000).await
}

async fn common_setup2(
    context: &TestContext,
    deposit_amount: u64,
    vault_funding: u64,
) -> CommonSetup {
    let admin = TestKeypair::new();
    let owner = context.users[0].key;
    let payer = context.users[1].key;
    let mints = &context.mints[0..3];

    let solana = &context.solana.clone();

    let group_with_tokens = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;
    let group = group_with_tokens.group;
    let tokens = group_with_tokens.tokens.clone();
    let base_token = &tokens[1];
    let quote_token = &tokens[0];

    //
    // SETUP: Create openbook market
    //
    let openbook_market_cookie = context
        .openbook
        .list_spot_market(&quote_token.mint, &base_token.mint, payer)
        .await;

    //
    // SETUP: Register openbook market
    //
    let openbook_v2_market = send_tx(
        solana,
        OpenbookV2RegisterMarketInstruction {
            group,
            admin,
            openbook_v2_program: context.openbook.program_id,
            openbook_v2_market_external: openbook_market_cookie.market,
            market_index: 0,
            base_bank: base_token.bank,
            quote_bank: quote_token.bank,
            payer,
        },
    )
    .await
    .unwrap()
    .openbook_v2_market;

    //
    // SETUP: Create accounts
    //
    let account = create_funded_account(
        &solana,
        group,
        owner,
        0,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    let account2 = create_funded_account(
        &solana,
        group,
        owner,
        2,
        &context.users[1],
        mints,
        deposit_amount,
        0,
    )
    .await;
    // to have enough funds in the vaults
    if vault_funding > 0 {
        create_funded_account(
            &solana,
            group,
            owner,
            3,
            &context.users[1],
            mints,
            10000000,
            0,
        )
        .await;
    }

    let open_orders = send_tx(
        solana,
        OpenbookV2CreateOpenOrdersInstruction {
            group,
            payer,
            owner,
            account,
            openbook_v2_market,
            openbook_v2_program: context.openbook.program_id,
            openbook_v2_market_external: openbook_market_cookie.market,
            next_open_orders_index: 1,
        },
    )
    .await
    .unwrap()
    .open_orders_account;

    let open_orders2 = send_tx(
        solana,
        OpenbookV2CreateOpenOrdersInstruction {
            group,
            payer,
            owner,
            account: account2,
            openbook_v2_market,
            openbook_v2_program: context.openbook.program_id,
            openbook_v2_market_external: openbook_market_cookie.market,
            next_open_orders_index: 1,
        },
    )
    .await
    .unwrap()
    .open_orders_account;

    let order_placer = OpenbookV2OrderPlacer {
        solana: solana.clone(),
        openbook: context.openbook.clone(),
        account,
        group,
        owner,
        openbook_market: openbook_v2_market,
        openbook_market_external: openbook_market_cookie.market,
        open_orders: open_orders,
        next_client_order_id: 420,
    };

    let order_placer2 = OpenbookV2OrderPlacer {
        solana: solana.clone(),
        openbook: context.openbook.clone(),
        account: account2,
        group,
        owner,
        openbook_market: openbook_v2_market,
        openbook_market_external: openbook_market_cookie.market,
        open_orders: open_orders2,
        next_client_order_id: 100000,
    };

    CommonSetup {
        group_with_tokens,
        openbook_market_cookie,
        quote_token: quote_token.clone(),
        base_token: base_token.clone(),
        order_placer,
        order_placer2,
    }
}
