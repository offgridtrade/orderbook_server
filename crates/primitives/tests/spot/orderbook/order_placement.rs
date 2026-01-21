use offgrid_primitives::spot::event::{self, SpotEvent};
use offgrid_primitives::spot::orderbook::OrderBook;
use super::EVENT_MUTEX;

fn lock_events() -> std::sync::MutexGuard<'static, ()> {
    EVENT_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

fn assert_order_placed(
    events: &event::EventQueue,
    expected_cid: Vec<u8>,
    expected_order_id: Vec<u8>,
    expected_owner: Vec<u8>,
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
        SpotEvent::SpotOrderPlaced {
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
        } if cid == &expected_cid
            && order_id == &expected_order_id
            && maker_account_id == &expected_owner
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


#[test]
fn place_ask_automatically_inserts_price_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_ask_automatically_inserts_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Verify price doesn't exist initially
    assert!(!orderbook.l2.price_exists(false, 100), "Price 100 should not exist initially");
    println!("Verified price 100 does not exist in ask prices");
    
    // Place ask order without manually inserting price first
    let _ = event::drain_events();
    let ask_order = orderbook.place_ask(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        1000,
        500,
        1234567890,
        i64::MAX,
        25,
    ).expect("place ask order should succeed");
    println!("Placed ask order with ID: {}, amount: 1000, price: 100", ask_order.id);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        ask_order.id.to_bytes().to_vec(),
        vec![10, 20],
        false,
        100,
        1000,
        500,
        500,
        1000,
        1234567890,
        i64::MAX,
    );
    
    // Verify price was automatically inserted
    assert!(orderbook.l2.price_exists(false, 100), "Price 100 should exist after placing ask order");
    println!("Verified price 100 was automatically inserted");
    
    // Verify price is in the ask head (if it's the only price, it should be the head)
    let ask_head = orderbook.l2.ask_head();
    assert_eq!(ask_head, Some(100), "Ask head should be 100");
    println!("Verified ask head is 100");
    
    // Verify level was set correctly (iceberg semantics: public = amnt - iqty = 1000 - 500 = 500)
    let ask_level = orderbook.l2.public_ask_level(100);
    assert_eq!(ask_level, Some(500), "public ask level should be 500 (iceberg-adjusted)");
    println!("Verified ask level is 1000");
    
    // Verify order exists
    let order = orderbook.l3.get_order(ask_order.id).expect("order should exist");
    assert_eq!(order.price, 100);
    assert_eq!(order.cqty, 1000);
    println!("Verified order details: price={}, cq={}", order.price, order.cqty);
    
    println!("Test passed: place_ask automatically inserts price and sets level correctly");
}

// Test that place_bid accumulates levels when multiple orders are placed at the same price

#[test]
fn place_ask_accumulates_levels_at_same_price_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_ask_accumulates_levels_at_same_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place first ask order
    let _ = event::drain_events();
    let ask_order_1 = orderbook.place_ask(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567890,
        12345678900,
        25,
    ).expect("place first ask order");
    let ask_order_id_1 = ask_order_1.id;
    println!("Placed first ask order with ID: {:?}, amount: 500", ask_order_id_1);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        ask_order_id_1.to_bytes().to_vec(),
        vec![10, 20],
        false,
        100,
        500,
        250,
        250,
        500,
        1234567890,
        12345678900,
    );
    
    // amnt=500, iqty=250, so pqty=250
    let level_after_first = orderbook.l2.public_ask_level(100);
    assert_eq!(level_after_first, Some(250), "public ask level should be 250 (iceberg-adjusted) after first order");
    println!("Verified level after first order: {:?}", level_after_first);
    
    // Place second ask order at the same price
    let ask_order_2 = orderbook.place_ask(
        vec![4, 5, 6],
        vec![0],
        vec![30, 40],
        100,
        300,
        150,
        1234567891,
        i64::MAX,
        25,
    ).expect("place second ask order");
    let ask_order_id_2 = ask_order_2.id;
    println!("Placed second ask order with ID: {:?}, amount: 300", ask_order_id_2);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![4, 5, 6],
        ask_order_id_2.to_bytes().to_vec(),
        vec![30, 40],
        false,
        100,
        300,
        150,
        150,
        300,
        1234567891,
        i64::MAX,
    );
    
    // First: amnt=500, iqty=250, pqty=250. Second: amnt=300, iqty=150, pqty=150. Total = 250+150=400
    let level_after_second = orderbook.l2.public_ask_level(100);
    assert_eq!(level_after_second, Some(400), "public ask level should be 400 (250 + 150, iceberg-adjusted) after second order");
    println!("Verified level after second order: {:?}", level_after_second);
    
    // Place third ask order at the same price
    let ask_order_3 = orderbook.place_ask(
        vec![7, 8, 9],
        vec![0],
        vec![50, 60],
        100,
        200,
        100,
        1234567892,
        i64::MAX,
        25,
    ).expect("place third ask order");
    let ask_order_id_3 = ask_order_3.id;
    println!("Placed third ask order with ID: {:?}, amount: 200", ask_order_id_3);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![7, 8, 9],
        ask_order_id_3.to_bytes().to_vec(),
        vec![50, 60],
        false,
        100,
        200,
        100,
        100,
        200,
        1234567892,
        i64::MAX,
    );
    
    // First: pqty=250, Second: pqty=150, Third: amnt=200, iqty=100, pqty=100. Total = 250+150+100=500
    let level_after_third = orderbook.l2.public_ask_level(100);
    assert_eq!(level_after_third, Some(500), "public ask level should be 500 (250+150+100, iceberg-adjusted) after third order");
    println!("Verified level after third order: {:?}", level_after_third);
    
    // Verify all orders exist
    let order1 = orderbook.l3.get_order(ask_order_id_1).expect("first order should exist");
    let order2 = orderbook.l3.get_order(ask_order_id_2).expect("second order should exist");
    let order3 = orderbook.l3.get_order(ask_order_id_3).expect("third order should exist");
    
    assert_eq!(order1.cqty, 500);
    assert_eq!(order2.cqty, 300);
    assert_eq!(order3.cqty, 200);
    println!("Verified all three orders exist with correct quantities");
    
    println!("Test passed: place_ask correctly accumulates levels at the same price");
}

// Test that place_bid handles multiple different prices correctly

#[test]
fn place_bid_accumulates_levels_at_same_price_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_bid_accumulates_levels_at_same_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place first bid order
    let _ = event::drain_events();
    let bid_order_1 = orderbook.place_bid(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567890,
        i64::MAX,
        25,
    ).expect("place first bid order");
    let bid_order_id_1 = bid_order_1.id;
    println!("Placed first bid order with ID: {}, amount: 500", bid_order_id_1);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        bid_order_id_1.to_bytes().to_vec(),
        vec![10, 20],
        true,
        100,
        500,
        250,
        250,
        500,
        1234567890,
        i64::MAX,
    );
    
    let level_after_first = orderbook.l2.public_bid_level(100);
    assert_eq!(level_after_first, Some(250), "Level should be 250 after first order");
    println!("Verified level after first order: {:?}", level_after_first);
    
    // Place second bid order at the same price
    let bid_order_2 = orderbook.place_bid(
        vec![4, 5, 6],
        vec![0],
        vec![30, 40],
        100,
        300,
        150,
        1234567891,
        i64::MAX,
        25,
    ).expect("place second bid order");
    let bid_order_id_2 = bid_order_2.id;
    println!("Placed second bid order with ID: {:?}, amount: 300", bid_order_id_2);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![4, 5, 6],
        bid_order_id_2.to_bytes().to_vec(),
        vec![30, 40],
        true,
        100,
        300,
        150,
        150,
        300,
        1234567891,
        i64::MAX,
    );
    
    let level_after_second = orderbook.l2.public_bid_level(100);
    assert_eq!(level_after_second, Some(400), "Level should be 400 (250 + 150) after second order");
    println!("Verified level after second order: {:?}", level_after_second);
    
    // Place third bid order at the same price
    let bid_order_3 = orderbook.place_bid(
        vec![7, 8, 9],
        vec![0],
        vec![50, 60],
        100,
        200,
        100,
        1234567892,
        i64::MAX,
        25,
    ).expect("place third bid order");
    let bid_order_id_3 = bid_order_3.id;
    println!("Placed third bid order with ID: {:?}, amount: 200", bid_order_id_3);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![7, 8, 9],
        bid_order_id_3.to_bytes().to_vec(),
        vec![50, 60],
        true,
        100,
        200,
        100,
        100,
        200,
        1234567892,
        i64::MAX,
    );
    
    let level_after_third = orderbook.l2.public_bid_level(100);
    assert_eq!(level_after_third, Some(500), "Level should be 500 (250 + 150 + 100) after third order");
    println!("Verified level after third order: {:?}", level_after_third);
    
    // Verify all orders exist
    let order1 = orderbook.l3.get_order(bid_order_id_1).expect("first order should exist");
    let order2 = orderbook.l3.get_order(bid_order_id_2).expect("second order should exist");
    let order3 = orderbook.l3.get_order(bid_order_id_3).expect("third order should exist");
    
    assert_eq!(order1.cqty, 500);
    assert_eq!(order2.cqty, 300);
    assert_eq!(order3.cqty, 200);
    println!("Verified all three orders exist with correct quantities");
    
    println!("Test passed: place_bid correctly accumulates levels at the same price");
}

// Test that place_ask accumulates levels when multiple orders are placed at the same price

#[test]
fn place_ask_handles_multiple_different_prices_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_ask_handles_multiple_different_prices");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place ask orders at different prices without manually inserting prices
    let _ = event::drain_events();
    let ask_order_1 = orderbook.place_ask(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567890,
        i64::MAX,
        25,
    ).expect("place ask order at 100");
    let ask_order_id_1 = ask_order_1.id;
    println!("Placed ask order at price 100, ID: {:?}, amount: 500", ask_order_id_1);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        ask_order_id_1.to_bytes().to_vec(),
        vec![10, 20],
        false,
        100,
        500,
        250,
        250,
        500,
        1234567890,
        i64::MAX,
    );
    
    let ask_order_2 = orderbook.place_ask(
        vec![4, 5, 6],
        vec![0],
        vec![30, 40],
        110,
        300,
        150,
        1234567891,
        i64::MAX,
        25,
    ).expect("place ask order at 110");
    let ask_order_id_2 = ask_order_2.id;
    println!("Placed ask order at price 110, ID: {:?}, amount: 300", ask_order_id_2);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![4, 5, 6],
        ask_order_id_2.to_bytes().to_vec(),
        vec![30, 40],
        false,
        110,
        300,
        150,
        150,
        300,
        1234567891,
        i64::MAX,
    );
    
    let ask_order_3 = orderbook.place_ask(
        vec![7, 8, 9],
        vec![0],
        vec![50, 60],
        95,
        200,
        100,
        1234567892,
        12345678902,
        25,
    ).expect("place ask order at 95");
    let ask_order_id_3 = ask_order_3.id;
    println!("Placed ask order at price 95, ID: {:?}, amount: 200", ask_order_id_3);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![7, 8, 9],
        ask_order_id_3.to_bytes().to_vec(),
        vec![50, 60],
        false,
        95,
        200,
        100,
        100,
        200,
        1234567892,
        12345678902,
    );
    
    // Verify all prices were inserted
    assert!(orderbook.l2.price_exists(false, 100), "Price 100 should exist");
    assert!(orderbook.l2.price_exists(false, 110), "Price 110 should exist");
    assert!(orderbook.l2.price_exists(false, 95), "Price 95 should exist");
    println!("Verified all three prices were automatically inserted");
    
    // Verify ask head is 95 (lowest price for asks)
    let ask_head = orderbook.l2.ask_head();
    assert_eq!(ask_head, Some(95), "Ask head should be 95 (lowest price)");
    println!("Verified ask head is 95");
    
    // Verify levels are set correctly (iceberg semantics: public = amnt - iqty)
    // 100: amnt=500, iqty=250, pqty=250
    // 110: amnt=300, iqty=150, pqty=150
    // 95: amnt=200, iqty=100, pqty=100
    assert_eq!(orderbook.l2.public_ask_level(100), Some(250));
    assert_eq!(orderbook.l2.public_ask_level(110), Some(150));
    assert_eq!(orderbook.l2.public_ask_level(95), Some(100));
    println!("Verified all levels are set correctly: 100={:?}, 110={:?}, 95={:?}", 
             orderbook.l2.public_ask_level(100), 
             orderbook.l2.public_ask_level(110), 
             orderbook.l2.public_ask_level(95));
    
    println!("Test passed: place_ask correctly handles multiple different prices");
}


// expired order on execute should be removed from the orderbook with event emitted

#[test]
fn set_iceberg_quantity_updates_public_level() {
    let _guard = lock_events();
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);

    let order = orderbook
        .place_bid(
            vec![1, 2, 3],
            vec![0],
            vec![10, 20],
            100,
            1000,
            500,
            1234567890,
            i64::MAX,
            25,
        )
        .expect("place bid order");
    let order_id = order.id;
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        order_id.to_bytes().to_vec(),
        vec![10, 20],
        true,
        100,
        1000,
        500,
        500,
        1000,
        1234567890,
        i64::MAX,
    );

    assert_eq!(orderbook.l2.public_bid_level(100), Some(500));
    assert_eq!(orderbook.l2.current_bid_level(100), Some(1000));

    orderbook
        .set_iceberg_quantity(vec![1, 2, 3], vec![0], true, order_id, 800)
        .expect("set iceberg quantity");
    let events = event::drain_events();
    let expected_order_id = order_id.to_bytes().to_vec();
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderIcebergQuantityChanged { order_id: event_order_id, .. }
            if event_order_id == &expected_order_id
    )));

    assert_eq!(orderbook.l2.public_bid_level(100), Some(200));
    assert_eq!(orderbook.l2.current_bid_level(100), Some(1000));
}

#[test]
fn place_ask_order_and_check_ask_price_level_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_ask_order_and_check_ask_price_level");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    let initial_level = orderbook.l2.public_ask_level(100);
    println!("Initial ask level at price 100: {:?}", initial_level);
    
    let ask_order = orderbook.place_ask(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        1000,
        500,
        1234567890,
        i64::MAX,
        25,
    ).expect("place ask order");
    let ask_order_id = ask_order.id;
    println!("Placed ask order with ID: {}, amount: 1000", ask_order_id);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        ask_order_id.to_bytes().to_vec(),
        vec![10, 20],
        false,
        100,
        1000,
        500,
        500,
        1000,
        1234567890,
        i64::MAX,
    );
    
    let updated_level = orderbook.l2.public_ask_level(100);
    println!("Updated ask level at price 100: {:?}", updated_level);
    assert_eq!(updated_level, Some(500), "Ask level should be updated to 500 (1000 - 500 iceberg) after placing order");
    println!("Test passed: ask price level correctly updated");
}

// execute a trade from ask order to bid order and check if the ask price level is updated

#[test]
fn place_bid_automatically_inserts_price_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_bid_automatically_inserts_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Verify price doesn't exist initially
    assert!(!orderbook.l2.price_exists(true, 100), "Price 100 should not exist initially");
    println!("Verified price 100 does not exist in bid prices");
    
    // Place bid order without manually inserting price first
    let _ = event::drain_events();
    let bid_order = orderbook.place_bid(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        1000,
        500,
        1234567890,
        i64::MAX,
        25,
    ).expect("place bid order should succeed");
    println!(
        "Placed bid order with ID: {}, amount: 1000, price: 100",
        bid_order.id
    );
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        bid_order.id.to_bytes().to_vec(),
        vec![10, 20],
        true,
        100,
        1000,
        500,
        500,
        1000,
        1234567890,
        i64::MAX,
    );
    
    // Verify price was automatically inserted
    assert!(orderbook.l2.price_exists(true, 100), "Price 100 should exist after placing bid order");
    println!("Verified price 100 was automatically inserted");
    
    // Verify price is in the bid head (if it's the only price, it should be the head)
    let bid_head = orderbook.l2.bid_head();
    assert_eq!(bid_head, Some(100), "Bid head should be 100");
    println!("Verified bid head is 100");
    
    // Verify level was set correctly
    let bid_level = orderbook.l2.public_bid_level(100);
    assert_eq!(bid_level, Some(500), "Bid level should be 500");
    println!("Verified bid level is 1000");
    
    // Verify order exists
    let order = orderbook.l3.get_order(bid_order.id).expect("order should exist");
    assert_eq!(order.price, 100);
    assert_eq!(order.cqty, 1000);
    println!("Verified order details: price={}, cq={}", order.price, order.cqty);
    
    println!("Test passed: place_bid automatically inserts price and sets level correctly");
}

// Test that place_ask automatically inserts price if it doesn't exist

#[test]
fn place_bid_handles_multiple_different_prices_without_expiration() {
    let _guard = lock_events();
    println!("Starting test: place_bid_handles_multiple_different_prices");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place bid orders at different prices without manually inserting prices
    let _ = event::drain_events();
    let bid_order_1 = orderbook.place_bid(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        500,
        250,
        1234567890,
        i64::MAX,
        25,
    ).expect("place bid order at 100");
    let bid_order_id_1 = bid_order_1.id;
    println!("Placed bid order at price 100, ID: {:?}, amount: 500", bid_order_id_1);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![1, 2, 3],
        bid_order_id_1.to_bytes().to_vec(),
        vec![10, 20],
        true,
        100,
        500,
        250,
        250,
        500,
        1234567890,
        i64::MAX,
    );
    
    let bid_order_2 = orderbook.place_bid(
        vec![4, 5, 6],
        vec![0],
        vec![30, 40],
        95,
        300,
        150,
        1234567891,
        i64::MAX,
        25,
    ).expect("place bid order at 95");
    let bid_order_id_2 = bid_order_2.id;
    println!("Placed bid order at price 95, ID: {:?}, amount: 300", bid_order_id_2);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![4, 5, 6],
        bid_order_id_2.to_bytes().to_vec(),
        vec![30, 40],
        true,
        95,
        300,
        150,
        150,
        300,
        1234567891,
        i64::MAX,
    );
    
    let bid_order_3 = orderbook.place_bid(
        vec![7, 8, 9],
        vec![0],
        vec![50, 60],
        105,
        200,
        100,
        1234567892,
        i64::MAX,
        25,
    ).expect("place bid order at 105");
    let bid_order_id_3 = bid_order_3.id;
    println!("Placed bid order at price 105, ID: {:?}, amount: 200", bid_order_id_3);
    let events = event::drain_events();
    assert_order_placed(
        &events,
        vec![7, 8, 9],
        bid_order_id_3.to_bytes().to_vec(),
        vec![50, 60],
        true,
        105,
        200,
        100,
        100,
        200,
        1234567892,
        i64::MAX,
    );
    
    // Verify all prices were inserted
    assert!(orderbook.l2.price_exists(true, 100), "Price 100 should exist");
    assert!(orderbook.l2.price_exists(true, 95), "Price 95 should exist");
    assert!(orderbook.l2.price_exists(true, 105), "Price 105 should exist");
    println!("Verified all three prices were automatically inserted");
    
    // Verify bid head is 105 (highest price)
    let bid_head = orderbook.l2.bid_head();
    assert_eq!(bid_head, Some(105), "Bid head should be 105 (highest price)");
    println!("Verified bid head is 105");
    
    // Verify levels are set correctly
    assert_eq!(orderbook.l2.public_bid_level(100), Some(250));
    assert_eq!(orderbook.l2.public_bid_level(95), Some(150));
    assert_eq!(orderbook.l2.public_bid_level(105), Some(100));
    println!("Verified all levels are set correctly: 100={:?}, 95={:?}, 105={:?}", 
             orderbook.l2.public_bid_level(100), 
             orderbook.l2.public_bid_level(95), 
             orderbook.l2.public_bid_level(105));
    
    println!("Test passed: place_bid correctly handles multiple different prices");
}

// Test that place_ask handles multiple different prices correctly
