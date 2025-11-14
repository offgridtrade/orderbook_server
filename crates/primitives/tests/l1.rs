use offgrid_primitives::market::L1;

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

