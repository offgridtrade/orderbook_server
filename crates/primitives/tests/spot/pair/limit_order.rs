use offgrid_primitives::spot::event;
use offgrid_primitives::spot::time_in_force::TimeInForce;
use offgrid_primitives::spot::Pair;

use super::EVENT_MUTEX;

fn lock_events() -> std::sync::MutexGuard<'static, ()> {
    EVENT_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

const SCALE_8: u64 = 1_0000_0000;

#[test]
fn limit_buy_moves_lmp_to_best_ask() {
    let _guard = lock_events();
    let mut pair = Pair::new();
    pair.pair_id = vec![1];
    pair.base_asset_id = vec![2];
    pair.quote_asset_id = vec![3];

    let _ = event::drain_events();

    let ask_price = 90 * SCALE_8;
    let _ask_order = pair
        .orderbook
        .place_ask(
            vec![1],
            pair.pair_id.clone(),
            pair.base_asset_id.clone(),
            pair.quote_asset_id.clone(),
            vec![10],
            ask_price,
            1 * SCALE_8,
            0,
            123,
            i64::MAX,
            5,
        )
        .expect("place ask");

    assert_eq!(pair.orderbook.lmp(), None);

    pair.limit_buy(
        vec![2],
        None,
        vec![20],
        100 * SCALE_8,
        100 * SCALE_8,
        0,
        124,
        i64::MAX,
        5,
        10,
        TimeInForce::GoodTillCanceled,
    )
    .expect("limit buy");

    assert_eq!(pair.orderbook.lmp(), Some(ask_price));
}

#[test]
fn limit_sell_moves_lmp_to_best_bid() {
    let _guard = lock_events();
    let mut pair = Pair::new();
    pair.pair_id = vec![4];
    pair.base_asset_id = vec![5];
    pair.quote_asset_id = vec![6];

    let _ = event::drain_events();

    let bid_price = 110 * SCALE_8;
    let _bid_order = pair
        .orderbook
        .place_bid(
            vec![3],
            pair.pair_id.clone(),
            pair.base_asset_id.clone(),
            pair.quote_asset_id.clone(),
            vec![30],
            bid_price,
            1 * SCALE_8,
            0,
            223,
            i64::MAX,
            5,
        )
        .expect("place bid");

    assert_eq!(pair.orderbook.lmp(), None);

    pair.limit_sell(
        vec![4],
        None,
        vec![40],
        100 * SCALE_8,
        100 * SCALE_8,
        0,
        224,
        i64::MAX,
        5,
        10,
        TimeInForce::GoodTillCanceled,
    )
    .expect("limit sell");

    assert_eq!(pair.orderbook.lmp(), Some(bid_price));
}
