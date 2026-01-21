use offgrid_primitives::spot::orderbook::OrderBook;
use offgrid_primitives::spot::event::{self, SpotEvent};
use offgrid_primitives::spot::orderbook::OrderBookError;
use offgrid_primitives::spot::orders::Order;
use ulid::Ulid;
use super::EVENT_MUTEX;

fn lock_events() -> std::sync::MutexGuard<'static, ()> {
    EVENT_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

fn assert_order_filled(
    events: &event::EventQueue,
    expected_cid: Vec<u8>,
    expected_order_id: Vec<u8>,
    expected_maker_account_id: Vec<u8>,
    expected_taker_account_id: Vec<u8>,
    expected_is_bid: bool,
    expected_price: u64,
    expected_pair_id: Vec<u8>,
    expected_base_asset_id: Vec<u8>,
    expected_quote_asset_id: Vec<u8>,
    expected_base_amount: u64,
    expected_quote_amount: u64,
    expected_base_fee: u64,
    expected_quote_fee: u64,
    expected_amnt: u64,
    expected_iqty: u64,
    expected_pqty: u64,
    expected_cqty: u64,
    expected_timestamp: i64,
    expected_expires_at: i64,
    allow_full: bool,
) {
    let matches_event = |e: &SpotEvent| matches!(
        e,
        SpotEvent::SpotOrderPartiallyFilled {
            cid,
            order_id,
            maker_account_id,
            taker_account_id,
            is_bid,
            price,
            pair_id,
            base_asset_id,
            quote_asset_id,
            base_amount,
            quote_amount,
            base_fee,
            quote_fee,
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
                && taker_account_id == &expected_taker_account_id
                && *is_bid == expected_is_bid
                && *price == expected_price
                && pair_id == &expected_pair_id
                && base_asset_id == &expected_base_asset_id
                && quote_asset_id == &expected_quote_asset_id
                && *base_amount == expected_base_amount
                && *quote_amount == expected_quote_amount
                && *base_fee == expected_base_fee
                && *quote_fee == expected_quote_fee
                && *amnt == expected_amnt
                && *iqty == expected_iqty
                && *pqty == expected_pqty
                && *cqty == expected_cqty
                && *timestamp == expected_timestamp
                && *expires_at == expected_expires_at
    );
    let matches_full = |e: &SpotEvent| matches!(
        e,
        SpotEvent::SpotOrderFullyFilled {
            cid,
            order_id,
            maker_account_id,
            taker_account_id,
            is_bid,
            price,
            pair_id,
            base_asset_id,
            quote_asset_id,
            base_amount,
            quote_amount,
            base_fee,
            quote_fee,
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
                && taker_account_id == &expected_taker_account_id
                && *is_bid == expected_is_bid
                && *price == expected_price
                && pair_id == &expected_pair_id
                && base_asset_id == &expected_base_asset_id
                && quote_asset_id == &expected_quote_asset_id
                && *base_amount == expected_base_amount
                && *quote_amount == expected_quote_amount
                && *base_fee == expected_base_fee
                && *quote_fee == expected_quote_fee
                && *amnt == expected_amnt
                && *iqty == expected_iqty
                && *pqty == expected_pqty
                && *cqty == expected_cqty
                && *timestamp == expected_timestamp
                && *expires_at == expected_expires_at
    );

    assert!(events.iter().any(|e| matches_event(e) || (allow_full && matches_full(e))));
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


#[test]
fn expired_order_on_pop_front_moves_to_next_price_level() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(110);

    let expired_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![10, 20],
            110, // best bid price
            1000,
            500,
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
            vec![30, 40],
            100, // next best bid price
            2000,
            1000,
            1234567891,
            i64::MAX,
            25,
        )
        .expect("place active bid order");
    let active_id = active_order.id;

    // check order is in price level
    assert_eq!(orderbook.l3.price_head.get(&110), Some(&expired_id));
    assert_eq!(orderbook.l3.price_tail.get(&110), Some(&expired_id));
    assert_eq!(orderbook.l3.price_head.get(&100), Some(&active_id));
    assert_eq!(orderbook.l3.price_tail.get(&100), Some(&active_id));

    // check public/current levels in l2
    // For bids, public level tracks the iceberg‐aware visible quantity,
    // whereas current level tracks the full order amount.
    assert_eq!(orderbook.l2.public_bid_level(110), Some(500));
    assert_eq!(orderbook.l2.current_bid_level(110), Some(1000));
    assert_eq!(orderbook.l2.public_bid_level(100), Some(1000));
    assert_eq!(orderbook.l2.current_bid_level(100), Some(2000));

    let popped = orderbook.pop_front(true).expect("pop front");
    assert_eq!(popped.id, active_id);
    assert!(orderbook.l3.get_order(expired_id).is_err());
    // After popping the only active order at price 100, the bid head
    // should be cleared since there are no remaining bid price levels.
    assert_eq!(orderbook.l2.bid_head(), None);

    let events = event::drain_events();
    assert_order_expired_without_timestamp(
        &events,
        vec![1, 2, 3],
        expired_id.to_bytes().to_vec(),
        vec![10, 20],
        true,
        110,
        1000,
        500,
        500,
        1000,
        0,
    );
}

#[test]
fn execute_trade_from_ask_order_to_bid_order_and_check_ask_price_level_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: execute_trade_from_ask_order_to_bid_order_and_check_ask_price_level");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    
    let ask_order = orderbook.place_ask(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567890,
        12345678900,
        25,
    ).expect("place ask order");
    println!("Placed ask order with ID: {}, amount: 500", ask_order.id);
    
    let ask_level_before = orderbook.l2.public_ask_level(100);
    let bid_level_before = orderbook.l2.public_bid_level(100);
    println!(
        "Before execution - Ask level: {:?}, Bid level: {:?}",
        ask_level_before, bid_level_before
    );

    // Clear any previous events
    let _ = event::drain_events();

    // Dummy taker bid order that will hit the resting ask
    let taker_order = Order::new(
        vec![9, 9, 9],    // cid
        Ulid::new(),      // id
        vec![7, 7, 7],    // owner
        true,             // is_bid (bid taker)
        100,              // price
        300,              // amnt
        0,                // iqty
        300,              // pqty
        300,              // cqty
        0,                // timestamp
        i64::MAX,         // expires_at
        25,               // fee_bps
    );

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
            false,                // update ask side in L2
            taker_order.clone(),  // taker
            ask_order.clone(),    // maker (resting ask)
            vec![0],              // pair_id
            vec![0],              // base_asset_id
            vec![0],              // quote_asset_id
            300,                  // Execute 300 out of 500
            false,                // clear: partial fill
            0,                    // now
        )
        .expect("execute trade");
    
    let ask_level_after = orderbook.l2.public_ask_level(100);
    let bid_level_after = orderbook.l2.public_bid_level(100);
    println!(
        "After execution - Ask level: {:?}, Bid level: {:?}",
        ask_level_after, bid_level_after
    );
    
    // With iceberg: initial pqty = 500 - 250 = 250, after executing 300 from cqty=500
    // The public level decreases based on the pqty change, which depends on how iceberg orders are handled
    assert_eq!(ask_level_after, Some(200), "Ask level should reflect iceberg-adjusted quantity after execution");
    assert_eq!(bid_level_after, None, "Bid level should remain None as no order was placed at this price");
    println!("Test passed: ask price level correctly updated after execution");

    let events = event::drain_events();
    let base_amount = 300;
    let quote_amount = (300 * 100) / 1_0000_0000;
    let base_fee = base_amount * 25 / 10000;
    let quote_fee = quote_amount * 25 / 10000;
    assert_order_filled(
        &events,
        vec![9, 9, 9],
        taker_order.id.to_bytes().to_vec(),
        vec![7, 7, 7],
        vec![10, 20],
        false,
        100,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        300,
        0,
        300,
        300,
        0,
        12345678900,
        true,
    );
    assert_order_filled(
        &events,
        vec![1, 2, 3],
        ask_order.id.to_bytes().to_vec(),
        vec![10, 20],
        vec![7, 7, 7],
        false,
        100,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        500,
        250,
        250,
        500,
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
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    println!("Inserted bid and ask prices at 100, set initial levels to 0");
    
    let bid_order = orderbook.place_bid(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567890,
        i64::MAX    ,
        25,
    ).expect("place bid order");
    let bid_order_id = bid_order.id;
    println!("Placed bid order with ID: {}, amount: 500", bid_order_id);
    
    let ask_order = orderbook.place_ask(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567891,
        i64::MAX,
        25,
    ).expect("place ask order");
    let ask_order_id = ask_order.id;
    println!("Placed ask order with ID: {}, amount: 500", ask_order_id);

    let bid_level_before = orderbook.l2.public_bid_level(100);
    let ask_level_before = orderbook.l2.public_ask_level(100);
    println!(
        "Before execution - Bid level: {:?}, Ask level: {:?}",
        bid_level_before, ask_level_before
    );

    // Clear any previous events
    let _ = event::drain_events();

    // Dummy taker order (not stored in L3) representing an incoming bid
    let taker_order = Order::new(
        vec![9, 9, 9],       // cid
        Ulid::new(),         // id
        vec![7, 7, 7],       // owner
        true,                // is_bid
        100,                 // price
        300,                 // amnt (we'll execute 300)
        0,                   // iqty
        300,                 // pqty
        300,                 // cqty
        0,                   // timestamp
        i64::MAX,            // expires_at
        25,                  // fee_bps
    );

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
            true,               // is_bid: true (bid taker)
            taker_order.clone(),
            bid_order.clone(),  // maker on the book
            vec![0],            // pair_id
            vec![0],            // base_asset_id
            vec![0],            // quote_asset_id
            300,                // amount to execute
            false,              // clear: partial fill
            0,                  // now
        )
        .expect("execute trade");

    let events = event::drain_events();
    let base_amount = (300 * 1_0000_0000) / 100;
    let quote_amount = 300;
    let base_fee = base_amount * 25 / 10000;
    let quote_fee = quote_amount * 25 / 10000;
    assert_order_filled(
        &events,
        vec![9, 9, 9],
        taker_order.id.to_bytes().to_vec(),
        vec![7, 7, 7],
        vec![10, 20],
        true,
        100,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        300,
        0,
        300,
        300,
        0,
        i64::MAX,
        true,
    );
    assert_order_filled(
        &events,
        vec![1, 2, 3],
        bid_order.id.to_bytes().to_vec(),
        vec![10, 20],
        vec![7, 7, 7],
        true,
        100,
        vec![0],
        vec![0],
        vec![0],
        base_amount,
        quote_amount,
        base_fee,
        quote_fee,
        500,
        250,
        250,
        500,
        0,
        i64::MAX,
        true,
    );
    
    let bid_level_after = orderbook.l2.public_bid_level(100);
    let ask_level_after = orderbook.l2.public_ask_level(100);
    println!(
        "After execution - Bid level: {:?}, Ask level: {:?}",
        bid_level_after, ask_level_after
    );
    
    // Bid: initial pqty = 500 - 250 = 250, after executing 300 from cqty=500
    // The public level decreases based on the pqty change
    assert_eq!(bid_level_after, Some(200), "Bid level should reflect iceberg-adjusted quantity after execution");
    // Ask side is untouched by this trade (only the resting bid is decremented), so public ask level stays the same
    assert_eq!(ask_level_after, Some(250), "Ask level should remain unchanged since the resting ask was not matched");
    println!("Test passed: bid price level correctly updated after execution");
}

// Test that place_bid automatically inserts price if it doesn't exist

#[test]
fn expired_order_on_execute_is_removed_and_emits_event() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);

    // Maker order (resting, expired)
    let bid_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![10, 20],
            100,
            1000,
            0,
            1234567890,
            0, // expired
            25,
        )
        .expect("place bid order");
    let bid_order_id = bid_order.id;

    // Dummy taker order (not stored in L3)
    let taker_order = Order::new(
        vec![9, 9, 9],           // cid
        Ulid::new(),            // id
        vec![7, 7, 7],          // owner
        true,                   // is_bid
        100,                    // price
        1000,                   // amnt
        0,                      // iqty
        1000,                   // pqty
        1000,                   // cqty
        0,                      // timestamp
        i64::MAX,               // expires_at
        25,                     // fee_bps
    );

    let result = orderbook.execute(
        true,
        taker_order,
        bid_order,
        vec![0],
        vec![0],
        vec![0],
        100,
        false,
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
        100,
        1000,
        0,
        1000,
        1000,
        1,
        0,
    );
}


// expired order on pop_front should be removed from the orderbook with event emitted and get another order from the same price level

#[test]
fn expired_order_on_pop_front_skips_to_next() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);

    let expired_order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![10, 20],
            100,
            1000,
            500,
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
            vec![30, 40],
            100,
            2000,
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
        100,
        1000,
        500,
        500,
        1000,
        0,
    );
}

// expired order on pop_front should move to next price level when the best price is emptied
