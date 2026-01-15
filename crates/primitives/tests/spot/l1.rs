use offgrid_primitives::spot::market::L1;

#[test]
fn updates_fields() {
    let mut l1 = L1::new();

    l1.lmp = Some(110);
    l1.bid_head = Some(120);
    l1.ask_head = Some(130);
    l1.limit_buy_slippage_limit = Some(9);
    l1.limit_sell_slippage_limit = Some(10);
    l1.market_buy_slippage_limit = Some(11);
    l1.market_sell_slippage_limit = Some(12);

    assert_eq!(l1.lmp, Some(110));
    assert_eq!(l1.bid_head, Some(120));
    assert_eq!(l1.ask_head, Some(130));
    assert_eq!(l1.limit_buy_slippage_limit, Some(9));
    assert_eq!(l1.limit_sell_slippage_limit, Some(10));
    assert_eq!(l1.market_buy_slippage_limit, Some(11));
    assert_eq!(l1.market_sell_slippage_limit, Some(12));
}

#[test]
fn updates_last_match_price() {
    let mut l1 = L1::new();
    l1.lmp = Some(105);
    assert_eq!(l1.lmp, Some(105));
}

#[test]
fn updates_heads() {
    let mut l1 = L1::new();
    l1.bid_head = Some(95);
    l1.ask_head = Some(115);
    assert_eq!(l1.bid_head, Some(95));
    assert_eq!(l1.ask_head, Some(115));
}


#[test]
fn updates_slippage_limits() {
    let mut l1 = L1::new();
    l1.limit_buy_slippage_limit = Some(6);
    l1.limit_sell_slippage_limit = Some(7);
    l1.market_buy_slippage_limit = Some(11);
    l1.market_sell_slippage_limit = Some(12);
    assert_eq!(l1.limit_buy_slippage_limit, Some(6));
    assert_eq!(l1.limit_sell_slippage_limit, Some(7));
    assert_eq!(l1.market_buy_slippage_limit, Some(11));
    assert_eq!(l1.market_sell_slippage_limit, Some(12));
}

#[test]
fn serialize_and_deserialize_l1() {
    let mut l1 = L1::new();
    l1.lmp = Some(110);
    l1.bid_head = Some(120);
    l1.ask_head = Some(130);
    l1.limit_buy_slippage_limit = Some(9);
    l1.limit_sell_slippage_limit = Some(10);
    l1.market_buy_slippage_limit = Some(11);
    l1.market_sell_slippage_limit = Some(12);

    // Serialize to binary format
    let encoded = postcard::to_allocvec(&l1).expect("serialize L1");
    
    // Deserialize from binary format
    let decoded: L1 = postcard::from_bytes(&encoded).expect("deserialize L1");
    
    // Verify all fields match
    assert_eq!(decoded.lmp, l1.lmp);
    assert_eq!(decoded.bid_head, l1.bid_head);
    assert_eq!(decoded.ask_head, l1.ask_head);
    assert_eq!(decoded.limit_buy_slippage_limit, l1.limit_buy_slippage_limit);
    assert_eq!(decoded.limit_sell_slippage_limit, l1.limit_sell_slippage_limit);
    assert_eq!(decoded.market_buy_slippage_limit, l1.market_buy_slippage_limit);
    assert_eq!(decoded.market_sell_slippage_limit, l1.market_sell_slippage_limit);
    assert_eq!(decoded, l1);
}

#[test]
fn serialize_and_deserialize_l1_with_none_values() {
    // Test with all None values
    let l1 = L1 {
        lmp: None,
        bid_head: None,
        ask_head: None,
        limit_buy_slippage_limit: None,
        limit_sell_slippage_limit: None,
        market_buy_slippage_limit: None,
        market_sell_slippage_limit: None,
    };

    // Serialize to binary format
    let encoded = postcard::to_allocvec(&l1).expect("serialize L1");
    
    // Deserialize from binary format
    let decoded: L1 = postcard::from_bytes(&encoded).expect("deserialize L1");
    
    // Verify all fields match
    assert_eq!(decoded, l1);
    assert_eq!(decoded.lmp, None);
    assert_eq!(decoded.bid_head, None);
    assert_eq!(decoded.ask_head, None);
}

#[test]
fn serialize_and_deserialize_l1_default() {
    // Test with default values
    let l1 = L1::default();

    // Serialize to binary format
    let encoded = postcard::to_allocvec(&l1).expect("serialize L1");
    
    // Deserialize from binary format
    let decoded: L1 = postcard::from_bytes(&encoded).expect("deserialize L1");
    
    // Verify all fields match
    assert_eq!(decoded, l1);
    assert_eq!(decoded.limit_buy_slippage_limit, Some(10000u64));
    assert_eq!(decoded.limit_sell_slippage_limit, Some(10000u64));
    assert_eq!(decoded.market_buy_slippage_limit, Some(10000u64));
    assert_eq!(decoded.market_sell_slippage_limit, Some(10000u64));
}
