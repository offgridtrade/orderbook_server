use offgrid_primitives::prices::{L2, Level};

#[test]
fn maintains_price_lists_and_levels() {
    let mut l2 = L2::default();

    l2.bid_price_lists.insert(100, 1);
    l2.ask_price_lists.insert(110, 1);

    l2.bids.push(Level {
        price: 100,
        quantity: 1_000,
    });
    l2.asks.push(Level {
        price: 110,
        quantity: 2_000,
    });

    assert_eq!(l2.bid_price_lists.get(&100), Some(&1));
    assert_eq!(l2.ask_price_lists.get(&110), Some(&1));
    assert_eq!(l2.bids.len(), 1);
    assert_eq!(l2.asks.len(), 1);
}

