use offgrid_primitives::orderbook::OrderBook;

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
fn serialize_and_deserialize_orderbook_with_orders() {
    let mut orderbook = OrderBook::new();
    
    // Set last matched price
    orderbook.set_lmp(100);
    
    // Insert prices into L2 and set initial levels before placing orders
    orderbook.l2.insert_price(true, 100).expect("insert bid price 100");
    orderbook.l2.set_bid_level(100, 0);
    orderbook.l2.insert_price(true, 95).expect("insert bid price 95");
    orderbook.l2.set_bid_level(95, 0);
    orderbook.l2.insert_price(false, 110).expect("insert ask price 110");
    orderbook.l2.set_ask_level(110, 0);
    orderbook.l2.insert_price(false, 115).expect("insert ask price 115");
    orderbook.l2.set_ask_level(115, 0);
    
    // Place some bid orders
    let (bid_order_id_1, _) = orderbook.place_bid(
        vec![1, 2, 3],
        vec![10, 20],
        100,
        1000,
        500,
        1234567890,
        12345678900,
        25,
    ).expect("place bid order 1");
    
    let (bid_order_id_2, _) = orderbook.place_bid(
        vec![4, 5, 6],
        vec![30, 40],
        95,
        2000,
        1000,
        1234567891,
        12345678901,
        30,
    ).expect("place bid order 2");
    
    // Place some ask orders
    let (ask_order_id_1, _) = orderbook.place_ask(
        vec![7, 8, 9],
        vec![50, 60],
        110,
        1500,
        750,
        1234567892,
        12345678902,
        25,
    ).expect("place ask order 1");
    
    let (ask_order_id_2, _) = orderbook.place_ask(
        vec![10, 11, 12],
        vec![70, 80],
        115,
        3000,
        1500,
        1234567893,
        12345678903,
        30,
    ).expect("place ask order 2");

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
    assert_eq!(bid_order_1.cq, 1000);
    assert_eq!(bid_order_2.price, 95);
    assert_eq!(bid_order_2.cq, 2000);
    assert_eq!(ask_order_1.price, 110);
    assert_eq!(ask_order_1.cq, 1500);
    assert_eq!(ask_order_2.price, 115);
    assert_eq!(ask_order_2.cq, 3000);
    
    // Verify L2 price levels are preserved
    assert_eq!(decoded.l2.bid_head(), orderbook.l2.bid_head());
    assert_eq!(decoded.l2.ask_head(), orderbook.l2.ask_head());
    assert_eq!(decoded.l2.bid_price_tail, orderbook.l2.bid_price_tail);
    assert_eq!(decoded.l2.ask_price_tail, orderbook.l2.ask_price_tail);
    
    // Verify complete equality
    assert_eq!(decoded, orderbook);
}

#[test]
fn serialize_and_deserialize_orderbook_after_execution() {
    let mut orderbook = OrderBook::new();
    
    // Set last matched price
    orderbook.set_lmp(100);
    
    // Insert prices into L2 and set initial levels before placing orders
    orderbook.l2.insert_price(true, 100).expect("insert bid price 100");
    orderbook.l2.set_bid_level(100, 0);
    orderbook.l2.insert_price(false, 100).expect("insert ask price 100");
    orderbook.l2.set_ask_level(100, 0);
    
    // Place a bid order
    let (bid_order_id, _) = orderbook.place_bid(
        vec![1, 2, 3],
        vec![10, 20],
        100,
        1000,
        500,
        1234567890,
        12345678900,
        25,
    ).expect("place bid order");
    
    // Place an ask order
    let (ask_order_id, _) = orderbook.place_ask(
        vec![4, 5, 6],
        vec![30, 40],
        100,
        500,
        250,
        1234567891,
        12345678901,
        25,
    ).expect("place ask order");
    
    // Execute a trade (decreases the ask order)
    let order_match = orderbook.execute(
        false, // is_bid: false (ask order)
        ask_order_id,
        300, // Execute 300 out of 500
        false, // clear: false (partial fill)
        25, // taker_fee_bps
    ).expect("execute trade");
    
    // Verify execution
    assert_eq!(order_match.base_amount, 300);
    
    // Serialize to binary format after execution
    let encoded = postcard::to_allocvec(&orderbook).expect("serialize OrderBook after execution");
    
    // Deserialize from binary format
    let decoded: OrderBook = postcard::from_bytes(&encoded).expect("deserialize OrderBook after execution");
    
    // Verify L1 is preserved (LMP should be updated to 100)
    assert_eq!(decoded.lmp(), orderbook.lmp());
    assert_eq!(decoded.lmp(), Some(100));
    
    // Verify orders are preserved with updated quantities
    let remaining_bid = decoded.l3.get_order(bid_order_id).expect("get remaining bid order");
    assert_eq!(remaining_bid.cq, 1000); // Bid order unchanged
    
    let remaining_ask = decoded.l3.get_order(ask_order_id).expect("get remaining ask order");
    assert_eq!(remaining_ask.cq, 200); // Ask order: 500 - 300 = 200
    
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
