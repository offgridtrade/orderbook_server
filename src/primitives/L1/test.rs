use super::L1;

#[test]
fn updates_last_match_price() {
    let mut l1 = L1::new(100, 90, 110, 5, 5, 10, 10);
    l1.lmp = 105;
    assert_eq!(l1.lmp, 105);
}

#[test]
fn updates_heads() {
    let mut l1 = L1::new(100, 90, 110, 5, 5, 10, 10);
    l1.bid_head = 95;
    l1.ask_head = 115;
    assert_eq!(l1.bid_head, 95);
    assert_eq!(l1.ask_head, 115);
}

#[test]
fn updates_slippage_limits() {
    let mut l1 = L1::new(100, 90, 110, 5, 5, 10, 10);
    l1.limit_buy_slippage_limit = 6;
    l1.limit_sell_slippage_limit = 7;
    l1.market_buy_slippage_limit = 11;
    l1.market_sell_slippage_limit = 12;

    assert_eq!(l1.limit_buy_slippage_limit, 6);
    assert_eq!(l1.limit_sell_slippage_limit, 7);
    assert_eq!(l1.market_buy_slippage_limit, 11);
    assert_eq!(l1.market_sell_slippage_limit, 12);
}