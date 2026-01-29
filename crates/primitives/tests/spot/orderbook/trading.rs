use super::EVENT_MUTEX;
use offgrid_primitives::spot::event::{self, SpotEvent};
use offgrid_primitives::spot::orderbook::OrderBook;
use offgrid_primitives::spot::orderbook::OrderBookError;
use offgrid_primitives::spot::orders::{Order, OrderId};
use ulid::Ulid;

fn lock_events() -> std::sync::MutexGuard<'static, ()> {
    EVENT_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

fn assert_order_filled(
    events: &event::EventQueue,
    expected_is_taker_event: bool,
    expected_taker_cid: Vec<u8>,
    expected_maker_cid: Vec<u8>,
    expected_taker_order_id: Vec<u8>,
    expected_maker_order_id: Vec<u8>,
    expected_taker_account_id: Vec<u8>,
    expected_maker_account_id: Vec<u8>,
    expected_taker_is_bid: bool,
    expected_maker_is_bid: bool,
    expected_price: u64,
    expected_pair_id: Vec<u8>,
    expected_base_asset_id: Vec<u8>,
    expected_quote_asset_id: Vec<u8>,
    expected_base_volume: u64,
    expected_quote_volume: u64,
    expected_base_fee: u64,
    expected_quote_fee: u64,
    expected_maker_fee_bps: u16,
    expected_taker_fee_bps: u16,
    expected_amnt: u64,
    expected_iqty: u64,
    expected_pqty: u64,
    expected_cqty: u64,
    expected_timestamp: i64,
    expected_expires_at: i64,
    allow_full: bool,
) {
    let matches_event = |e: &SpotEvent| {
        matches!(
            e,
            SpotEvent::SpotOrderPartiallyFilled {
                is_taker_event,
                taker_cid,
                maker_cid,
                taker_order_id,
                maker_order_id,
                taker_account_id,
                maker_account_id,
                taker_order_is_bid,
                maker_order_is_bid,
                price,
                pair_id,
                base_asset_id,
                quote_asset_id,
                base_volume,
                quote_volume,
                base_fee,
                quote_fee,
                maker_fee_bps,
                taker_fee_bps,
                amnt,
                iqty,
                pqty,
                cqty,
                timestamp,
                expires_at,
            }
                if *is_taker_event == expected_is_taker_event
                    && taker_cid == &expected_taker_cid
                    && maker_cid == &expected_maker_cid
                    && taker_order_id == &expected_taker_order_id
                    && maker_order_id == &expected_maker_order_id
                    && taker_account_id == &expected_taker_account_id
                    && maker_account_id == &expected_maker_account_id
                    && *taker_order_is_bid == expected_taker_is_bid
                    && *maker_order_is_bid == expected_maker_is_bid
                    && *price == expected_price
                    && pair_id == &expected_pair_id
                    && base_asset_id == &expected_base_asset_id
                    && quote_asset_id == &expected_quote_asset_id
                    && *base_volume == expected_base_volume
                    && *quote_volume == expected_quote_volume
                    && *base_fee == expected_base_fee
                    && *quote_fee == expected_quote_fee
                    && *maker_fee_bps == expected_maker_fee_bps
                    && *taker_fee_bps == expected_taker_fee_bps
                    && *amnt == expected_amnt
                    && *iqty == expected_iqty
                    && *pqty == expected_pqty
                    && *cqty == expected_cqty
                    && *timestamp == expected_timestamp
                    && *expires_at == expected_expires_at
        )
    };
    let matches_full = |e: &SpotEvent| {
        matches!(
            e,
            SpotEvent::SpotOrderFullyFilled {
                is_taker_event,
                taker_cid,
                maker_cid,
                taker_order_id,
                maker_order_id,
                taker_account_id,
                maker_account_id,
                taker_order_is_bid,
                maker_order_is_bid,
                price,
                pair_id,
                base_asset_id,
                quote_asset_id,
                base_volume,
                quote_volume,
                base_fee,
                quote_fee,
                maker_fee_bps,
                taker_fee_bps,
                amnt,
                iqty,
                pqty,
                cqty,
                timestamp,
                expires_at,
            }
                if *is_taker_event == expected_is_taker_event
                    && taker_cid == &expected_taker_cid
                    && maker_cid == &expected_maker_cid
                    && taker_order_id == &expected_taker_order_id
                    && maker_order_id == &expected_maker_order_id
                    && taker_account_id == &expected_taker_account_id
                    && maker_account_id == &expected_maker_account_id
                    && *taker_order_is_bid == expected_taker_is_bid
                    && *maker_order_is_bid == expected_maker_is_bid
                    && *price == expected_price
                    && pair_id == &expected_pair_id
                    && base_asset_id == &expected_base_asset_id
                    && quote_asset_id == &expected_quote_asset_id
                    && *base_volume == expected_base_volume
                    && *quote_volume == expected_quote_volume
                    && *base_fee == expected_base_fee
                    && *quote_fee == expected_quote_fee
                    && *maker_fee_bps == expected_maker_fee_bps
                    && *taker_fee_bps == expected_taker_fee_bps
                    && *amnt == expected_amnt
                    && *iqty == expected_iqty
                    && *pqty == expected_pqty
                    && *cqty == expected_cqty
                    && *timestamp == expected_timestamp
                    && *expires_at == expected_expires_at
        )
    };

    assert!(events
        .iter()
        .any(|e| matches_event(e) || (allow_full && matches_full(e))));
}

fn assert_order_expired(
    events: &event::EventQueue,
    expected_cid: Vec<u8>,
    expected_order_id: Vec<u8>,
    expected_maker_account_id: Vec<u8>,
    expected_is_bid: bool,
    expected_price: u64,
    expected_amnt: u64,
    expected_iqty: u64,
    expected_pqty: u64,
    expected_cqty: u64,
    expected_timestamp: i64,
    expected_expires_at: i64,
) {
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderExpired {
            cid,
            order_id,
            maker_account_id,
            is_bid,
            price,
            amnt,
            iqty,
            pqty,
            cqty,
            timestamp,
            expires_at,
        }
            if cid == &expected_cid
                && order_id == &expected_order_id
                && maker_account_id == &expected_maker_account_id
                && *is_bid == expected_is_bid
                && *price == expected_price
                && *amnt == expected_amnt
                && *iqty == expected_iqty
                && *pqty == expected_pqty
                && *cqty == expected_cqty
                && *timestamp == expected_timestamp
                && *expires_at == expected_expires_at
    )));
}

fn assert_order_expired_without_timestamp(
    events: &event::EventQueue,
    expected_cid: Vec<u8>,
    expected_order_id: Vec<u8>,
    expected_maker_account_id: Vec<u8>,
    expected_is_bid: bool,
    expected_price: u64,
    expected_amnt: u64,
    expected_iqty: u64,
    expected_pqty: u64,
    expected_cqty: u64,
    expected_expires_at: i64,
) {
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderExpired {
            cid,
            order_id,
            maker_account_id,
            is_bid,
            price,
            amnt,
            iqty,
            pqty,
            cqty,
            expires_at,
            ..
        }
            if cid == &expected_cid
                && order_id == &expected_order_id
                && maker_account_id == &expected_maker_account_id
                && *is_bid == expected_is_bid
                && *price == expected_price
                && *amnt == expected_amnt
                && *iqty == expected_iqty
                && *pqty == expected_pqty
                && *cqty == expected_cqty
                && *expires_at == expected_expires_at
    )));
}

fn matching_amounts(orderbook: &OrderBook, taker: &Order, maker: &Order) -> (u64, u64, u64) {
    let taker_converted_matching_cqty = if taker.is_bid {
        taker
            .cqty
            .saturating_mul(1_0000_0000)
            .saturating_div(taker.price)
    } else {
        taker
            .cqty
            .saturating_mul(taker.price)
            .saturating_div(1_0000_0000)
    };

    let matching_amount = if taker_converted_matching_cqty >= maker.cqty {
        orderbook
            .get_required(maker.clone(), taker.price, maker.cqty)
            .expect("taker amount from maker")
    } else if taker_converted_matching_cqty < maker.cqty {
        orderbook
            .get_required(taker.clone(), maker.price, taker.cqty)
            .expect("maker amount from taker")
    } else {
        taker.cqty
    };

    let matching_base_amount = if taker.is_bid {
        matching_amount
            .saturating_mul(1_0000_0000)
            .saturating_div(taker.price)
    } else {
        matching_amount
    };
    let matching_quote_amount = if taker.is_bid {
        matching_amount
    } else {
        matching_amount
            .saturating_mul(taker.price)
            .saturating_div(1_0000_0000)
    };

    (matching_amount, matching_base_amount, matching_quote_amount)
}

fn remaining_quantities(orderbook: &OrderBook, order_id: OrderId) -> (u64, u64) {
    match orderbook.l3.get_order(order_id) {
        Ok(order) => (order.pqty, order.cqty),
        Err(_) => (0, 0),
    }
}

#[test]
fn expired_order_on_pop_front_moves_to_next_price_level() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(110 * 1_0000_0000);

    let expired_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![0],
            vec![0],
            vec![10, 20],
            110 * 1_0000_0000, // best bid price
            1000 * 1_0000_0000,
            500 * 1_0000_0000,
            1234567890,
            0, // expired
            25,
        )
        .expect("place expired bid order");
    let expired_id = expired_order.id;
    let active_order = orderbook
        .place_bid(
            vec![4, 5, 6],
            vec![0],
            vec![0],
            vec![0],
            vec![30, 40],
            100 * 1_0000_0000, // next best bid price
            2000 * 1_0000_0000,
            1000 * 1_0000_0000,
            1234567891,
            i64::MAX,
            25,
        )
        .expect("place active bid order");
    let active_id = active_order.id;

    // check order is in price level
    assert_eq!(
        orderbook.l3.price_head.get(&(110 * 1_0000_0000)),
        Some(&expired_id)
    );
    assert_eq!(
        orderbook.l3.price_tail.get(&(110 * 1_0000_0000)),
        Some(&expired_id)
    );
    assert_eq!(
        orderbook.l3.price_head.get(&(100 * 1_0000_0000)),
        Some(&active_id)
    );
    assert_eq!(
        orderbook.l3.price_tail.get(&(100 * 1_0000_0000)),
        Some(&active_id)
    );

    // check public/current levels in l2
    // For bids, public level tracks the iceberg‐aware visible quantity,
    // whereas current level tracks the full order amount.
    assert_eq!(
        orderbook.l2.public_bid_level(110 * 1_0000_0000),
        Some(500 * 1_0000_0000)
    );
    assert_eq!(
        orderbook.l2.current_bid_level(110 * 1_0000_0000),
        Some(1000 * 1_0000_0000)
    );
    assert_eq!(
        orderbook.l2.public_bid_level(100 * 1_0000_0000),
        Some(1000 * 1_0000_0000)
    );
    assert_eq!(
        orderbook.l2.current_bid_level(100 * 1_0000_0000),
        Some(2000 * 1_0000_0000)
    );

    let popped = orderbook.pop_front(true).expect("pop front");
    assert_eq!(popped.id, active_id);
    assert!(orderbook.l3.get_order(expired_id).is_err());
    // After popping the only active order at price 100, the bid head
    // should be cleared since there are no remaining bid price levels.
    assert_eq!(orderbook.l2.bid_head(), None);

    let events = event::drain_events();
    println!("events len: {}", events.len());
    for e in events.iter() {
        println!("event: {:?}", e);
    }
    println!("events len: {}", events.len());
    for e in events.iter() {
        println!("event: {:?}", e);
    }
    assert_order_expired_without_timestamp(
        &events,
        vec![1, 2, 3],
        expired_id.to_bytes().to_vec(),
        vec![10, 20],
        true,
        110 * 1_0000_0000,
        1000 * 1_0000_0000,
        500 * 1_0000_0000,
        500 * 1_0000_0000,
        1000 * 1_0000_0000,
        0,
    );
}

#[test]
fn execute_trade_from_ask_order_to_bid_order_and_check_ask_price_level_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: execute_trade_from_ask_order_to_bid_order_and_check_ask_price_level");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100 * 1_0000_0000);
    println!("Set LMP to 100");

    let ask_order = orderbook
        .place_ask(
            vec![1, 2, 3],
            vec![0],
            vec![0],
            vec![0],
            vec![10, 20],
            100 * 1_0000_0000,
            500 * 1_0000_0000,
            0,
            1234567890,
            12345678900,
            25,
        )
        .expect("place ask order");
    println!("Placed ask order with ID: {}, amount: 500", ask_order.id);

    let ask_level_before = orderbook.l2.public_ask_level(100 * 1_0000_0000);
    let bid_level_before = orderbook.l2.public_bid_level(100 * 1_0000_0000);
    println!(
        "Before execution - Ask level: {:?}, Bid level: {:?}",
        ask_level_before, bid_level_before
    );

    // Clear any previous events
    let _ = event::drain_events();

    let taker_order = orderbook
        .place_bid(
            vec![9, 9, 9],
            vec![0],
            vec![0],
            vec![0],
            vec![7, 7, 7],
            100 * 1_0000_0000,
            500 * 1_0000_0000,
            0,
            0,
            i64::MAX,
            25,
        )
        .expect("place taker bid");

    // Configure fee recipients so fee events can be emitted without panicking
    orderbook
        .fee_recipients
        .insert(ask_order.cid.clone(), b"ask_admin".to_vec());
    orderbook
        .fee_recipients
        .insert(taker_order.cid.clone(), b"taker_admin".to_vec());

    // Execute the trade
    orderbook
        .execute(
            taker_order.clone(), // taker
            ask_order.clone(),   // maker (resting ask)
            vec![0],
            // pair_id
            vec![0],
            // base_asset_id
            vec![0],
            // quote_asset_id
            0,                 // now
        )
        .expect("execute trade");

    let ask_level_after = orderbook.l2.public_ask_level(100 * 1_0000_0000);
    let bid_level_after = orderbook.l2.public_bid_level(100 * 1_0000_0000);
    println!(
        "After execution - Ask level: {:?}, Bid level: {:?}",
        ask_level_after, bid_level_after
    );

    let (taker_pqty, taker_cqty) = remaining_quantities(&orderbook, taker_order.id);
    let (maker_pqty, maker_cqty) = remaining_quantities(&orderbook, ask_order.id);
    if maker_pqty == 0 {
        assert!(ask_level_after.is_none() || ask_level_after == Some(0));
    } else {
        assert_eq!(ask_level_after, Some(maker_pqty));
    }
    if taker_pqty == 0 {
        assert!(bid_level_after.is_none() || bid_level_after == Some(0));
    } else {
        assert_eq!(bid_level_after, Some(taker_pqty));
    }
    println!("Test passed: ask price level correctly updated after execution");

    let events = event::drain_events();
    let (_, base_amount, quote_amount) = matching_amounts(&orderbook, &taker_order, &ask_order);
    let base_fee = base_amount * 25 / 10000;
    let quote_fee = quote_amount * 25 / 10000;
    println!(
        "Transfer plan: base {} from taker to maker; quote {} from maker to taker",
        base_amount, quote_amount
    );
    println!(
        "Taker owner {:?} -> maker owner {:?} (base), maker owner {:?} -> taker owner {:?} (quote)",
        taker_order.owner, ask_order.owner, ask_order.owner, taker_order.owner
    );
    assert_order_filled(
        &events,
        true,
        vec![9, 9, 9],
        vec![1, 2, 3],
        taker_order.id.to_bytes().to_vec(),
        ask_order.id.to_bytes().to_vec(),
        vec![7, 7, 7],
        vec![10, 20],
        true,
        false,
        100 * 1_0000_0000,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        ask_order.fee_bps,
        taker_order.fee_bps,
        taker_order.amnt,
        taker_order.iqty,
        taker_pqty,
        taker_cqty,
        0,
        i64::MAX,
        true,
    );
    assert_order_filled(
        &events,
        false,
        vec![9, 9, 9],
        vec![1, 2, 3],
        taker_order.id.to_bytes().to_vec(),
        ask_order.id.to_bytes().to_vec(),
        vec![7, 7, 7],
        vec![10, 20],
        true,
        false,
        100 * 1_0000_0000,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        ask_order.fee_bps,
        taker_order.fee_bps,
        ask_order.amnt,
        ask_order.iqty,
        maker_pqty,
        maker_cqty,
        0,
        12345678900,
        true,
    );
}

// execute a trade from bid order to ask order and check if the bid price level is updated

#[test]
fn execute_trade_from_bid_order_to_ask_order_and_check_bid_price_level_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: execute_trade_from_bid_order_to_ask_order_and_check_bid_price_level");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100 * 1_0000_0000);
    println!("Set LMP to 100");

    println!("Inserted bid and ask prices at 100, set initial levels to 0");

    let bid_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![0],
            vec![0],
            vec![10, 20],
            100 * 1_0000_0000,
            1000 * 1_0000_0000,
            0,
            1234567890,
            i64::MAX,
            25,
        )
        .expect("place bid order");
    let bid_order_id = bid_order.id;
    println!("Placed bid order with ID: {}, amount: 500", bid_order_id);

    let taker_order = orderbook
        .place_ask(
            vec![9, 9, 9],
            vec![0],
            vec![0],
            vec![0],
            vec![7, 7, 7],
            100 * 1_0000_0000,
            500 * 1_0000_0000,
            0,
            1234567891,
            i64::MAX,
            25,
        )
        .expect("place taker ask");
    println!("Placed taker ask order with ID: {}, amount: 500", taker_order.id);

    let bid_level_before = orderbook.l2.public_bid_level(100 * 1_0000_0000);
    let ask_level_before = orderbook.l2.public_ask_level(100 * 1_0000_0000);
    println!(
        "Before execution - Bid level: {:?}, Ask level: {:?}",
        bid_level_before, ask_level_before
    );

    // Clear any previous events
    let _ = event::drain_events();

    // Configure fee recipients for taker and maker so fee events can be emitted
    orderbook
        .fee_recipients
        .insert(bid_order.cid.clone(), b"bid_admin".to_vec());
    orderbook
        .fee_recipients
        .insert(taker_order.cid.clone(), b"taker_admin".to_vec());

    // Execute the trade via events (no OrderMatch return any more)
    orderbook
        .execute(
            taker_order.clone(),
            bid_order.clone(), // maker on the book
            vec![0],
            // pair_id
            vec![0],
            // base_asset_id
            vec![0],
            // quote_asset_id
            0,                 // now
        )
        .expect("execute trade");

    let events = event::drain_events();
    let (_, base_amount, quote_amount) = matching_amounts(&orderbook, &taker_order, &bid_order);
    let base_fee = base_amount * 25 / 10000;
    let quote_fee = quote_amount * 25 / 10000;
    let (taker_pqty, taker_cqty) = remaining_quantities(&orderbook, taker_order.id);
    let (maker_pqty, maker_cqty) = remaining_quantities(&orderbook, bid_order.id);
    assert_order_filled(
        &events,
        true,
        vec![9, 9, 9],
        vec![1, 2, 3],
        taker_order.id.to_bytes().to_vec(),
        bid_order.id.to_bytes().to_vec(),
        vec![7, 7, 7],
        vec![10, 20],
        false,
        true,
        100 * 1_0000_0000,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        bid_order.fee_bps,
        taker_order.fee_bps,
        taker_order.amnt,
        taker_order.iqty,
        taker_pqty,
        taker_cqty,
        0,
        i64::MAX,
        true,
    );
    assert_order_filled(
        &events,
        false,
        vec![9, 9, 9],
        vec![1, 2, 3],
        taker_order.id.to_bytes().to_vec(),
        bid_order.id.to_bytes().to_vec(),
        vec![7, 7, 7],
        vec![10, 20],
        false,
        true,
        100 * 1_0000_0000,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        bid_order.fee_bps,
        taker_order.fee_bps,
        bid_order.amnt,
        bid_order.iqty,
        maker_pqty,
        maker_cqty,
        0,
        i64::MAX,
        true,
    );

    let bid_level_after = orderbook.l2.public_bid_level(100 * 1_0000_0000);
    let ask_level_after = orderbook.l2.public_ask_level(100 * 1_0000_0000);
    println!(
        "After execution - Bid level: {:?}, Ask level: {:?}",
        bid_level_after, ask_level_after
    );

    if maker_pqty == 0 {
        assert!(bid_level_after.is_none() || bid_level_after == Some(0));
    } else {
        assert_eq!(bid_level_after, Some(maker_pqty));
    }
    if taker_pqty == 0 {
        assert!(ask_level_after.is_none() || ask_level_after == Some(0));
    } else {
        assert_eq!(ask_level_after, Some(taker_pqty));
    }
    println!("Test passed: bid price level correctly updated after execution");
}

// Test that place_bid automatically inserts price if it doesn't exist

#[test]
fn expired_order_on_execute_is_removed_and_emits_event() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100 * 1_0000_0000);

    // Maker order (resting, expired)
    let bid_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![0],
            vec![0],
            vec![10, 20],
            100 * 1_0000_0000,
            1000 * 1_0000_0000,
            0,
            1234567890,
            0, // expired
            25,
        )
        .expect("place bid order");
    let bid_order_id = bid_order.id;

    // Dummy taker order (not stored in L3)
    let taker_order = Order::new(
        vec![9, 9, 9],
        // cid
        Ulid::new(), // id
        vec![7, 7, 7],
        // owner
        false,              // is_bid
        100 * 1_0000_0000,  // price
        1000 * 1_0000_0000, // amnt
        0,                  // iqty
        1000 * 1_0000_0000, // pqty
        1000 * 1_0000_0000, // cqty
        0,                  // timestamp
        i64::MAX,           // expires_at
        25,                 // fee_bps
    );

    let result = orderbook.execute(
        taker_order,
        bid_order,
        vec![0],
        vec![0],
        vec![0],
        1,
    );
    assert!(matches!(result, Err(OrderBookError::OrderExpired)));
    assert!(orderbook.l3.get_order(bid_order_id).is_err());

    let events = event::drain_events();
    assert_order_expired(
        &events,
        vec![1, 2, 3],
        bid_order_id.to_bytes().to_vec(),
        vec![10, 20],
        true,
        100 * 1_0000_0000,
        1000 * 1_0000_0000,
        0,
        1000 * 1_0000_0000,
        1000 * 1_0000_0000,
        1,
        0,
    );
}

// expired order on pop_front should be removed from the orderbook with event emitted and get another order from the same price level

#[test]
fn expired_order_on_pop_front_skips_to_next() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100 * 1_0000_0000);

    let expired_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![0],
            vec![0],
            vec![10, 20],
            100 * 1_0000_0000,
            1000 * 1_0000_0000,
            500 * 1_0000_0000,
            1234567890,
            0, // expired
            25,
        )
        .expect("place expired bid order");
    let expired_id = expired_order.id;
    let active_order = orderbook
        .place_bid(
            vec![4, 5, 6],
            vec![0],
            vec![0],
            vec![0],
            vec![30, 40],
            100 * 1_0000_0000,
            2000 * 1_0000_0000,
            0,
            1234567891,
            i64::MAX,
            25,
        )
        .expect("place active bid order");
    let active_id = active_order.id;

    let popped = orderbook.pop_front(true).expect("pop front");
    assert_eq!(popped.id, active_id);
    assert!(orderbook.l3.get_order(expired_id).is_err());

    let events = event::drain_events();
    assert_order_expired_without_timestamp(
        &events,
        vec![1, 2, 3],
        expired_id.to_bytes().to_vec(),
        vec![10, 20],
        true,
        100 * 1_0000_0000,
        1000 * 1_0000_0000,
        500 * 1_0000_0000,
        500 * 1_0000_0000,
        1000 * 1_0000_0000,
        0,
    );
}

// expired order on pop_front should move to next price level when the best price is emptied
