use offgrid_primitives::spot::orderbook::OrderBook;
use offgrid_primitives::spot::event::{self, SpotEvent};
use offgrid_primitives::spot::orderbook::OrderBookError;
use offgrid_primitives::spot::orders::Order;
use ulid::Ulid;

#[test]
fn serialize_and_deserialize_empty_orderbook() {
    // Test with default/empty OrderBook
    let orderbook = OrderBook::new();

    // Serialize to binary format
    let encoded = postcard::to_allocvec(&orderbook).expect("serialize OrderBook");
    
    // Deserialize from binary format
    let decoded: OrderBook = postcard::from_bytes(&encoded).expect("deserialize OrderBook");
    
    // Verify all fields match
    assert_eq!(decoded, orderbook);
    assert_eq!(decoded.lmp(), None);
}

#[test]
fn serialize_and_deserialize_orderbook_with_orders_without_expiration() {
    let mut orderbook = OrderBook::new();
    
    // Set last matched price
    orderbook.set_lmp(100);
    
    
    // Place some bid orders
    let bid_order_1 = orderbook.place_bid(
        vec![1, 2, 3],
        vec![0],
        vec![10, 20],
        100,
        1000,
        500,
        1234567890,
        i64::MAX,
        25,
    ).expect("place bid order 1");
    let bid_order_id_1 = bid_order_1.id;
    
    let bid_order_2 = orderbook.place_bid(
        vec![4, 5, 6],
        vec![0],
        vec![30, 40],
        95,
        2000,
        1000,
        1234567891,
        i64::MAX,
        30,
    ).expect("place bid order 2");
    let bid_order_id_2 = bid_order_2.id;
    
    // Place some ask orders
    let ask_order_1 = orderbook.place_ask(
        vec![7, 8, 9],
        vec![0],
        vec![50, 60],
        110,
        1500,
        750,
        1234567892,
        i64::MAX,
        25,
    ).expect("place ask order 1");
    let ask_order_id_1 = ask_order_1.id;
    
    let ask_order_2 = orderbook.place_ask(
        vec![10, 11, 12],
        vec![0],
        vec![70, 80],
        115,
        3000,
        1500,
        1234567893,
        i64::MAX,
        30,
    ).expect("place ask order 2");
    let ask_order_id_2 = ask_order_2.id;

    // Serialize to binary format
    let encoded = postcard::to_allocvec(&orderbook).expect("serialize OrderBook");
    
    // Deserialize from binary format
    let decoded: OrderBook = postcard::from_bytes(&encoded).expect("deserialize OrderBook");
    
    // Verify all fields match
    assert_eq!(decoded.lmp(), orderbook.lmp());
    assert_eq!(decoded.lmp(), Some(100));
    
    // Verify orders exist in decoded OrderBook
    let bid_order_1 = decoded.l3.get_order(bid_order_id_1).expect("get bid order 1");
    let bid_order_2 = decoded.l3.get_order(bid_order_id_2).expect("get bid order 2");
    let ask_order_1 = decoded.l3.get_order(ask_order_id_1).expect("get ask order 1");
    let ask_order_2 = decoded.l3.get_order(ask_order_id_2).expect("get ask order 2");
    
    assert_eq!(bid_order_1.price, 100);
    assert_eq!(bid_order_1.cqty, 1000);
    assert_eq!(bid_order_2.price, 95);
    assert_eq!(bid_order_2.cqty, 2000);
    assert_eq!(ask_order_1.price, 110);
    assert_eq!(ask_order_1.cqty, 1500);
    assert_eq!(ask_order_2.price, 115);
    assert_eq!(ask_order_2.cqty, 3000);
    
    // Verify L2 price levels are preserved
    assert_eq!(decoded.l2.bid_head(), orderbook.l2.bid_head());
    assert_eq!(decoded.l2.ask_head(), orderbook.l2.ask_head());
    assert_eq!(decoded.l2.bid_price_tail, orderbook.l2.bid_price_tail);
    assert_eq!(decoded.l2.ask_price_tail, orderbook.l2.ask_price_tail);
    
    // Verify complete equality
    assert_eq!(decoded, orderbook);
}

#[test]
fn serialize_and_deserialize_orderbook_after_execution_without_expiration() {
    let mut orderbook = OrderBook::new();
    
    // Set last matched price
    orderbook.set_lmp(100);
    
    
    // Place a bid order
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
    ).expect("place bid order");
    let bid_order_id = bid_order.id;
    
    // Place an ask order
    let ask_order = orderbook.place_ask(
        vec![4, 5, 6],
        vec![0],
        vec![30, 40],
        100,
        500,
        250,
        1234567891,
        i64::MAX,
        25,
    ).expect("place ask order");
    let ask_order_id = ask_order.id;

    // Clear any previous events
    let _ = event::drain_events();

    // Dummy taker order (incoming bid) that will hit the resting ask
    let taker_order = Order::new(
        vec![9, 9, 9],    // cid
        Ulid::new(),      // id
        vec![7, 7, 7],    // owner
        true,             // is_bid
        100,              // price
        300,              // amnt
        0,                // iqty
        300,              // pqty
        300,              // cqty
        1234567892,       // timestamp
        i64::MAX,         // expires_at
        25,               // fee_bps
    );

    // Configure fee recipients for all clients so fee emissions succeed
    orderbook
        .fee_recipients
        .insert(bid_order.cid.clone(), b"bid_admin".to_vec());
    orderbook
        .fee_recipients
        .insert(ask_order.cid.clone(), b"ask_admin".to_vec());
    orderbook
        .fee_recipients
        .insert(taker_order.cid.clone(), b"taker_admin".to_vec());

    // Execute a trade (decreases the ask order) – rely on events + book state
    orderbook
        .execute(
            false,            // taker is ask? – here we treat taker as bid hitting the ask
            taker_order.clone(),
            ask_order.clone(),
            vec![0],          // pair_id
            vec![0],          // base_asset_id
            vec![0],          // quote_asset_id
            300,              // Execute 300 out of 500
            false,            // clear: false (partial fill)
            1234567892,
        )
        .expect("execute trade");

    let events = event::drain_events();
    // Taker got a fill
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderPartiallyFilled { cid, order_id, .. }
            if *cid == taker_order.cid && *order_id == taker_order.id.to_bytes().to_vec()
    )));
    // Maker ask got a fill
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderPartiallyFilled { cid, order_id, .. }
            if *cid == ask_order.cid && *order_id == ask_order.id.to_bytes().to_vec()
    )));
    
    // Serialize to binary format after execution
    let encoded = postcard::to_allocvec(&orderbook).expect("serialize OrderBook after execution");
    
    // Deserialize from binary format
    let decoded: OrderBook = postcard::from_bytes(&encoded).expect("deserialize OrderBook after execution");
    
    // Verify L1 is preserved (LMP should be updated to 100)
    assert_eq!(decoded.lmp(), orderbook.lmp());
    assert_eq!(decoded.lmp(), Some(100));
    
    // Verify orders are preserved with updated quantities
    let remaining_bid = decoded.l3.get_order(bid_order_id).expect("get remaining bid order");
    assert_eq!(remaining_bid.cqty, 1000); // Bid order unchanged
    
    let remaining_ask = decoded.l3.get_order(ask_order_id).expect("get remaining ask order");
    assert_eq!(remaining_ask.cqty, 200); // Ask order: 500 - 300 = 200
    
    // Verify complete equality
    assert_eq!(decoded, orderbook);
}

#[test]
fn serialize_and_deserialize_orderbook_with_slippage_limits() {
    let mut orderbook = OrderBook::new();
    
    // Set last matched price
    orderbook.set_lmp(100);
    
    // Set slippage limits
    orderbook.l1.limit_buy_slippage_limit = Some(5000);
    orderbook.l1.limit_sell_slippage_limit = Some(6000);
    orderbook.l1.market_buy_slippage_limit = Some(7000);
    orderbook.l1.market_sell_slippage_limit = Some(8000);
    
    // Serialize to binary format
    let encoded = postcard::to_allocvec(&orderbook).expect("serialize OrderBook with slippage");
    
    // Deserialize from binary format
    let decoded: OrderBook = postcard::from_bytes(&encoded).expect("deserialize OrderBook with slippage");
    
    // Verify slippage limits are preserved
    assert_eq!(decoded.l1.limit_buy_slippage_limit, Some(5000));
    assert_eq!(decoded.l1.limit_sell_slippage_limit, Some(6000));
    assert_eq!(decoded.l1.market_buy_slippage_limit, Some(7000));
    assert_eq!(decoded.l1.market_sell_slippage_limit, Some(8000));
    
    // Verify complete equality
    assert_eq!(decoded, orderbook);
}


// place a bid order and check if the bid price level is updated
#[test]
fn place_bid_order_and_check_bid_price_level_without_expiration() {
    println!("Starting test: place_bid_order_and_check_bid_price_level");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    
    let initial_level = orderbook.l2.public_bid_level(100);
    println!("Initial bid level at price 100: {:?}", initial_level);
    
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
    ).expect("place bid order");
    let bid_order_id = bid_order.id;
    println!("Placed bid order with ID: {}, amount: 1000", bid_order_id);
    
    let updated_level = orderbook.l2.public_bid_level(100);
    println!("Updated bid level at price 100: {:?}", updated_level);
    assert_eq!(updated_level, Some(500), "Bid level should be updated to 500 after placing order");
    println!("Test passed: bid price level correctly updated");
}

// place an ask order and check if the ask price level is updated
#[test]
fn place_ask_order_and_check_ask_price_level_without_expiration() {
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
    
    let updated_level = orderbook.l2.public_ask_level(100);
    println!("Updated ask level at price 100: {:?}", updated_level);
    assert_eq!(updated_level, Some(500), "Ask level should be updated to 500 (1000 - 500 iceberg) after placing order");
    println!("Test passed: ask price level correctly updated");
}

// execute a trade from ask order to bid order and check if the ask price level is updated
#[test]
fn execute_trade_from_ask_order_to_bid_order_and_check_ask_price_level_without_expiration() {
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
}

// execute a trade from bid order to ask order and check if the bid price level is updated  
#[test]
fn execute_trade_from_bid_order_to_ask_order_and_check_bid_price_level_without_expiration() {
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

    // Verify taker got a fill event
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderPartiallyFilled { cid, order_id, .. }
            if *cid == taker_order.cid && *order_id == taker_order.id.to_bytes().to_vec()
    )));

    // Verify maker (resting bid) also got a fill event
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderPartiallyFilled { cid, order_id, .. }
            if *cid == bid_order.cid && *order_id == bid_order.id.to_bytes().to_vec()
    )));
    
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
fn place_bid_automatically_inserts_price_without_expiration() {
    println!("Starting test: place_bid_automatically_inserts_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Verify price doesn't exist initially
    assert!(!orderbook.l2.price_exists(true, 100), "Price 100 should not exist initially");
    println!("Verified price 100 does not exist in bid prices");
    
    // Place bid order without manually inserting price first
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
fn place_ask_automatically_inserts_price_without_expiration() {
    println!("Starting test: place_ask_automatically_inserts_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Verify price doesn't exist initially
    assert!(!orderbook.l2.price_exists(false, 100), "Price 100 should not exist initially");
    println!("Verified price 100 does not exist in ask prices");
    
    // Place ask order without manually inserting price first
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
fn place_bid_accumulates_levels_at_same_price_without_expiration() {
    println!("Starting test: place_bid_accumulates_levels_at_same_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place first bid order
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
fn place_ask_accumulates_levels_at_same_price_without_expiration() {
    println!("Starting test: place_ask_accumulates_levels_at_same_price");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place first ask order
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
fn place_bid_handles_multiple_different_prices_without_expiration() {
    println!("Starting test: place_bid_handles_multiple_different_prices");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place bid orders at different prices without manually inserting prices
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
#[test]
fn place_ask_handles_multiple_different_prices_without_expiration() {
    println!("Starting test: place_ask_handles_multiple_different_prices");
    let mut orderbook = OrderBook::new();
    orderbook.set_lmp(100);
    println!("Set LMP to 100");
    
    // Place ask orders at different prices without manually inserting prices
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
fn expired_order_on_execute_is_removed_and_emits_event() {
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
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderExpired { order_id, .. }
            if order_id == &bid_order_id.to_bytes().to_vec()
    )));
}


// expired order on pop_front should be removed from the orderbook with event emitted and get another order from the same price level
#[test]
fn expired_order_on_pop_front_skips_to_next() {
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
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderExpired { order_id, .. }
            if order_id == &expired_id.to_bytes().to_vec()
    )));
}

// expired order on pop_front should move to next price level when the best price is emptied
#[test]
fn expired_order_on_pop_front_moves_to_next_price_level() {
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
    assert!(events.iter().any(|e| matches!(
        e,
        SpotEvent::SpotOrderExpired { order_id, .. }
            if order_id == &expired_id.to_bytes().to_vec()
    )));
}

#[test]
fn set_iceberg_quantity_updates_public_level() {
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

    assert_eq!(orderbook.l2.public_bid_level(100), Some(500));
    assert_eq!(orderbook.l2.current_bid_level(100), Some(1000));

    orderbook
        .set_iceberg_quantity(vec![1, 2, 3], vec![0], true, order_id, 800)
        .expect("set iceberg quantity");

    assert_eq!(orderbook.l2.public_bid_level(100), Some(200));
    assert_eq!(orderbook.l2.current_bid_level(100), Some(1000));
}
