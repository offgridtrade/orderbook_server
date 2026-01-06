use offgrid_primitives::matching_engine::MatchingEngine;
use offgrid_primitives::orderbook::OrderBook;

/// Helper function to create a test orderbook with some initial orders
fn setup_orderbook_with_asks() -> OrderBook {
    let mut orderbook = OrderBook::new();
    
    // Create ask orders at price 100 (3 orders with amounts 50, 30, 20 = 100 total)
    let (id1, _) = orderbook.place_ask(
        "ask1", "alice", 100, 50, 50, 0, 10000, 10
    ).expect("place ask 1");
    orderbook.l3.insert_id(100, id1, 50).expect("insert ask 1");
    orderbook.l2.insert_price(false, 100).expect("insert ask price");
    
    let (id2, _) = orderbook.place_ask(
        "ask2", "bob", 100, 30, 30, 0, 10000, 10
    ).expect("place ask 2");
    orderbook.l3.insert_id(100, id2, 30).expect("insert ask 2");
    
    let (id3, _) = orderbook.place_ask(
        "ask3", "carol", 100, 20, 20, 0, 10000, 10
    ).expect("place ask 3");
    orderbook.l3.insert_id(100, id3, 20).expect("insert ask 3");
    
    // Create ask order at price 110
    let (id4, _) = orderbook.place_ask(
        "ask4", "dave", 110, 40, 40, 0, 10000, 10
    ).expect("place ask 4");
    orderbook.l3.insert_id(110, id4, 40).expect("insert ask 4");
    orderbook.l2.insert_price(false, 110).expect("insert ask price 110");
    
    orderbook
}

fn setup_orderbook_with_bids() -> OrderBook {
    let mut orderbook = OrderBook::new();
    
    // Create bid orders at price 90 (3 orders with amounts 50, 30, 20 = 100 total)
    let (id1, _) = orderbook.place_bid(
        "bid1", "alice", 90, 50, 50, 0, 10000, 10
    ).expect("place bid 1");
    orderbook.l3.insert_id(90, id1, 50).expect("insert bid 1");
    orderbook.l2.insert_price(true, 90).expect("insert bid price");
    
    let (id2, _) = orderbook.place_bid(
        "bid2", "bob", 90, 30, 30, 0, 10000, 10
    ).expect("place bid 2");
    orderbook.l3.insert_id(90, id2, 30).expect("insert bid 2");
    
    let (id3, _) = orderbook.place_bid(
        "bid3", "carol", 90, 20, 20, 0, 10000, 10
    ).expect("place bid 3");
    orderbook.l3.insert_id(90, id3, 20).expect("insert bid 3");
    
    // Create bid order at price 80
    let (id4, _) = orderbook.place_bid(
        "bid4", "dave", 80, 40, 40, 0, 10000, 10
    ).expect("place bid 4");
    orderbook.l3.insert_id(80, id4, 40).expect("insert bid 4");
    orderbook.l2.insert_price(true, 80).expect("insert bid price 80");
    
    orderbook
}

#[test]
fn test_match_at_single_order_full_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 100, 50, 50, 0, 10000, 20
    ).expect("place taker bid");
    
    // Match at price 100, should fully match first order (50)
    let remaining = engine._match_at(
        taker_id,
        100,
        50, // amount to match
        20, // taker fee bps
        true, // matching against asks
    ).expect("match_at should succeed");
    
    assert_eq!(remaining, 0, "Should fully match 50");
    
    // Check that first maker order is consumed
    let order1 = engine.orderbook.l3.get_order(1);
    assert!(order1.is_err() || order1.unwrap().cq == 0, "First order should be consumed");
}

#[test]
fn test_match_at_multiple_orders_partial_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 100, 70, 70, 0, 10000, 20
    ).expect("place taker bid");
    
    // Match at price 100, should match first order (50) and part of second (20)
    let remaining = engine._match_at(
        taker_id,
        100,
        70, // amount to match
        20, // taker fee bps
        true, // matching against asks
    ).expect("match_at should succeed");
    
    assert_eq!(remaining, 0, "Should fully match 70 (50 + 20)");
    
    // Check that first order is consumed, second order has remaining
    let order1 = engine.orderbook.l3.get_order(1);
    assert!(order1.is_err() || order1.unwrap().cq == 0, "First order should be fully consumed");
    
    let order2 = engine.orderbook.l3.get_order(2).expect("order 2 should exist");
    assert_eq!(order2.cq, 10, "Second order should have 10 remaining (30 - 20)");
}

#[test]
fn test_match_at_price_level_empty() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 100, 100, 100, 0, 10000, 20
    ).expect("place taker bid");
    
    // Match at price 100, should match all orders (50 + 30 + 20 = 100)
    let remaining = engine._match_at(
        taker_id,
        100,
        100, // amount to match
        20, // taker fee bps
        true, // matching against asks
    ).expect("match_at should succeed");
    
    assert_eq!(remaining, 0, "Should fully match 100");
    
    // Check that price level is empty
    assert!(engine.orderbook.l3.is_empty(100), "Price level 100 should be empty");
}

#[test]
fn test_limit_order_buy_full_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 105, 50, 50, 0, 10000, 20
    ).expect("place taker bid");
    
    // Limit buy at 105, should match against ask at 100
    let (remaining, _bid_head, ask_head) = engine._limit_order(
        taker_id,
        50, // amount
        true, // is_bid (buy)
        105, // limit_price
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    assert_eq!(remaining, 0, "Should fully match");
    // After matching 50, price 100 still has orders (30+20=50 remaining), so head should still be 100
    assert_eq!(ask_head, 100, "Price 100 should still have orders, so head should remain 100");
}

#[test]
fn test_limit_order_buy_partial_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 105, 75, 75, 0, 10000, 20
    ).expect("place taker bid");
    
    // Limit buy at 105, should match against ask at 100 (50 + 25 = 75)
    let (remaining, _bid_head, ask_head) = engine._limit_order(
        taker_id,
        75, // amount
        true, // is_bid (buy)
        105, // limit_price
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    assert_eq!(remaining, 0, "Should fully match 75");
    assert_eq!(ask_head, 100, "Price 100 should still have orders");
    
    // Check that second order has remaining (30 - 25 = 5)
    let order2 = engine.orderbook.l3.get_order(2).expect("order 2 should exist");
    assert_eq!(order2.cq, 5, "Second order should have 5 remaining");
}

#[test]
fn test_limit_order_buy_across_multiple_prices() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 115, 120, 120, 0, 10000, 20
    ).expect("place taker bid");
    
    // Limit buy at 115, should match against asks at 100 (100 total) and 110 (20)
    let (remaining, _bid_head, ask_head) = engine._limit_order(
        taker_id,
        120, // amount
        true, // is_bid (buy)
        115, // limit_price (can match up to 115)
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    assert_eq!(remaining, 0, "Should fully match 120");
    assert_eq!(ask_head, 110, "Price 110 should still have orders (40 - 20 = 20)");
    
    // Check that price 100 is empty
    assert!(engine.orderbook.l3.is_empty(100), "Price level 100 should be empty");
    
    // Check that order at 110 has remaining (40 - 20 = 20)
    let order4 = engine.orderbook.l3.get_order(4).expect("order 4 should exist");
    assert_eq!(order4.cq, 20, "Order at 110 should have 20 remaining");
}

#[test]
fn test_limit_order_sell_full_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_bids();
    
    // Create a taker ask order
    let (taker_id, _) = engine.orderbook.place_ask(
        "taker", "taker", 85, 50, 50, 0, 10000, 20
    ).expect("place taker ask");
    
    // Limit sell at 85, should match against bid at 90
    let (remaining, bid_head, _ask_head) = engine._limit_order(
        taker_id,
        50, // amount
        false, // is_bid (sell)
        85, // limit_price
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    assert_eq!(remaining, 0, "Should fully match");
    assert_eq!(bid_head, 90, "Price 90 should still have orders");
}

#[test]
fn test_limit_order_price_limit_not_met() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order with limit price below market
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 95, 50, 50, 0, 10000, 20
    ).expect("place taker bid");
    
    // Limit buy at 95, but asks are at 100, so should not match
    let (remaining, _bid_head, ask_head) = engine._limit_order(
        taker_id,
        50, // amount
        true, // is_bid (buy)
        95, // limit_price (below ask price 100)
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    assert_eq!(remaining, 50, "Should not match (limit price too low)");
    assert_eq!(ask_head, 100, "Ask head should still be 100");
}

#[test]
fn test_limit_order_empty_orderbook() {
    let mut engine = MatchingEngine::new();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 100, 50, 50, 0, 10000, 20
    ).expect("place taker bid");
    
    // Limit buy with empty orderbook
    let (remaining, bid_head, ask_head) = engine._limit_order(
        taker_id,
        50, // amount
        true, // is_bid (buy)
        100, // limit_price
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    assert_eq!(remaining, 50, "Should not match (no orders)");
    assert_eq!(bid_head, 0, "No bid head");
    assert_eq!(ask_head, 0, "No ask head");
}

#[test]
fn test_limit_order_lmp_set() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Create a taker bid order
    let (taker_id, _) = engine.orderbook.place_bid(
        "taker", "taker", 105, 50, 50, 0, 10000, 20
    ).expect("place taker bid");
    
    // Limit buy at 105, should match against ask at 100
    let (_remaining, _bid_head, _ask_head) = engine._limit_order(
        taker_id,
        50, // amount
        true, // is_bid (buy)
        105, // limit_price
        20, // taker_fee_bps
    ).expect("limit_order should succeed");
    
    // Check that lmp is set to 100 (the matched price)
    assert_eq!(engine.orderbook.lmp(), Some(100), "LMP should be set to matched price");
}

// Integration tests for limit_buy and limit_sell with matching
#[test]
fn test_limit_buy_integration_full_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Place a limit buy order - it should match immediately against asks
    let (order_id, _found_dormant) = engine.limit_buy(
        "buy1", "buyer", 105, // limit price 105
        50, // amount
        50, // public_amount
        0, // timestamp
        10000, // expires_at
        10, // maker_fee_bps
        20, // taker_fee_bps
    ).expect("limit_buy should succeed");
    
    // Order should be fully matched and removed
    let order_result = engine.orderbook.l3.get_order(order_id);
    assert!(order_result.is_err(), "Order should be fully matched and removed");
    
    // Check that the ask order at price 100 was matched
    let order1 = engine.orderbook.l3.get_order(1);
    assert!(order1.is_err() || order1.unwrap().cq == 0, "First ask order should be consumed");
}

#[test]
fn test_limit_buy_integration_partial_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Place a limit buy order for 75 - should match 50 from first order and 25 from second
    let (order_id, _found_dormant) = engine.limit_buy(
        "buy1", "buyer", 105, // limit price 105
        75, // amount
        75, // public_amount
        0, // timestamp
        10000, // expires_at
        10, // maker_fee_bps
        20, // taker_fee_bps
    ).expect("limit_buy should succeed");
    
    // Order should be fully matched and removed (we matched 75 total)
    let order_result = engine.orderbook.l3.get_order(order_id);
    assert!(order_result.is_err(), "Order should be fully matched and removed");
    
    // Check that first order is consumed, second order has remaining
    let order1 = engine.orderbook.l3.get_order(1);
    assert!(order1.is_err() || order1.unwrap().cq == 0, "First ask order should be fully consumed");
    
    let order2 = engine.orderbook.l3.get_order(2).expect("order 2 should exist");
    assert_eq!(order2.cq, 5, "Second order should have 5 remaining (30 - 25)");
}

#[test]
fn test_limit_sell_integration_full_match() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_bids();
    
    // Place a limit sell order - it should match immediately against bids
    let (order_id, _found_dormant) = engine.limit_sell(
        "sell1", "seller", 85, // limit price 85
        50, // amount
        50, // public_amount
        0, // timestamp
        10000, // expires_at
        10, // maker_fee_bps
        20, // taker_fee_bps
    ).expect("limit_sell should succeed");
    
    // Order should be fully matched and removed
    let order_result = engine.orderbook.l3.get_order(order_id);
    assert!(order_result.is_err(), "Order should be fully matched and removed");
    
    // Check that the bid order at price 90 was matched
    let order1 = engine.orderbook.l3.get_order(1);
    assert!(order1.is_err() || order1.unwrap().cq == 0, "First bid order should be consumed");
}

#[test]
fn test_limit_buy_no_match_price_too_low() {
    let mut engine = MatchingEngine::new();
    engine.orderbook = setup_orderbook_with_asks();
    
    // Place a limit buy order with price below market - should not match
    let (order_id, _found_dormant) = engine.limit_buy(
        "buy1", "buyer", 95, // limit price 95 (below ask at 100)
        50, // amount
        50, // public_amount
        0, // timestamp
        10000, // expires_at
        10, // maker_fee_bps
        20, // taker_fee_bps
    ).expect("limit_buy should succeed");
    
    // Order should remain in the orderbook (not matched)
    let order = engine.orderbook.l3.get_order(order_id).expect("Order should still exist");
    assert_eq!(order.cq, 50, "Order should not be matched, cq should be 50");
    
    // All ask orders should be intact
    let order1 = engine.orderbook.l3.get_order(1).expect("order 1 should exist");
    assert_eq!(order1.cq, 50, "Ask order should not be matched");
}

