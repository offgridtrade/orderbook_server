use offgrid_primitives::spot::event;
use offgrid_primitives::spot::Pair;
use super::EVENT_MUTEX;

fn lock_events() -> std::sync::MutexGuard<'static, ()> {
    EVENT_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn serialize_and_deserialize_empty_pair() {
    let _guard = lock_events();
    let pair = Pair::new();
    let encoded = postcard::to_allocvec(&pair).expect("serialize empty Pair");
    let decoded: Pair = postcard::from_bytes(&encoded).expect("deserialize empty Pair");
    assert_eq!(decoded, pair);
}

#[test]
fn serialize_and_deserialize_pair_with_orders() {
    let _guard = lock_events();
    let mut pair = Pair::new();
    pair.pair_id = vec![1];
    pair.base_asset_id = vec![2];
    pair.quote_asset_id = vec![3];

    pair.add_client(vec![9], vec![10], vec![11]);
    pair.add_client(vec![8], vec![12], vec![13]);

    let _ = event::drain_events();

    let bid_order = pair
        .orderbook
        .place_bid(
            vec![9],
            pair.pair_id.clone(),
            pair.base_asset_id.clone(),
            pair.quote_asset_id.clone(),
            vec![21],
            100,
            1000,
            500,
            1234567890,
            i64::MAX,
            25,
        )
        .expect("place bid order");

    let ask_order = pair
        .orderbook
        .place_ask(
            vec![8],
            pair.pair_id.clone(),
            pair.base_asset_id.clone(),
            pair.quote_asset_id.clone(),
            vec![22],
            110,
            800,
            400,
            1234567891,
            i64::MAX,
            25,
        )
        .expect("place ask order");

    let encoded = postcard::to_allocvec(&pair).expect("serialize Pair with orders");
    let decoded: Pair = postcard::from_bytes(&encoded).expect("deserialize Pair with orders");

    assert_eq!(decoded.pair_id, pair.pair_id);
    assert_eq!(decoded.base_asset_id, pair.base_asset_id);
    assert_eq!(decoded.quote_asset_id, pair.quote_asset_id);
    assert_eq!(decoded.clients, pair.clients);
    assert_eq!(decoded.client_admin_account_ids, pair.client_admin_account_ids);
    assert_eq!(decoded.client_fee_account_ids, pair.client_fee_account_ids);

    let decoded_bid = decoded
        .orderbook
        .l3
        .get_order(bid_order.id)
        .expect("bid order exists");
    let decoded_ask = decoded
        .orderbook
        .l3
        .get_order(ask_order.id)
        .expect("ask order exists");
    assert_eq!(decoded_bid.price, 100);
    assert_eq!(decoded_bid.cqty, 1000);
    assert_eq!(decoded_ask.price, 110);
    assert_eq!(decoded_ask.cqty, 800);
}