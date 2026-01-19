use offgrid_primitives::spot::orders::L3;

#[test]
fn remove_dormant_orders_removes_expired() {
    let mut l3 = L3::new();

    let now = 1_000;
    let (expired_id, _) = l3
        .create_order(vec![1], vec![2], 100, 10, 0, 0, now - 1, 10)
        .expect("create expired order");
    let (active_id, _) = l3
        .create_order(vec![3], vec![4], 200, 20, 0, 0, now + 100, 10)
        .expect("create active order");

    let removed = l3.remove_dormant_orders(now);

    let removed_ids: Vec<_> = removed.iter().map(|(id, _)| *id).collect();
    assert!(removed_ids.contains(&expired_id));
    assert!(!removed_ids.contains(&active_id));
    assert!(l3.get_order(expired_id).is_err());
    assert!(l3.get_order(active_id).is_ok());
}
