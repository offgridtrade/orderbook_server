use offgrid_primitives::spot::event::{self, SpotEvent};
use offgrid_primitives::spot::orderbook::OrderBook;
use offgrid_primitives::spot::orders::Order;
use ulid::Ulid;
use super::EVENT_MUTEX;

fn lock_events() -> std::sync::MutexGuard<'static, ()> {
    EVENT_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}


#[test]
fn serialize_and_deserialize_orderbook_with_slippage_limits() {
    let _guard = lock_events();
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
fn serialize_and_deserialize_orderbook_with_orders_without_expiration() {
    let _guard = lock_events();
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
fn serialize_and_deserialize_empty_orderbook() {
    let _guard = lock_events();
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
fn place_bid_order_and_check_bid_price_level_without_expiration() {
    let _guard = lock_events();
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
fn serialize_and_deserialize_orderbook_after_execution_without_expiration() {
    let _guard = lock_events();
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
