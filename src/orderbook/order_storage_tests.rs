use super::order_storage::OrderStorage;

fn setup_orders() -> OrderStorage {
    let mut storage = OrderStorage::new();
    let (id1, _) = storage
        .create_order(vec![1], vec![65, 108, 105, 99, 101], 100, 50, 50, 0)
        .unwrap();
    storage.insert_id(100, id1, 50).unwrap();

    let (id2, _) = storage
        .create_order(vec![2], vec![66, 111, 98], 100, 75, 75, 0)
        .unwrap();
    storage.insert_id(100, id2, 75).unwrap();

    let (id3, _) = storage
        .create_order(vec![3], vec![67, 97, 114, 111, 108], 100, 20, 20, 0)
        .unwrap();
    storage.insert_id(100, id3, 20).unwrap();

    storage
}

#[test]
fn inserts_orders_fifo() {
    let storage = setup_orders();
    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 3);

    let first_order = storage.get_order(ids[0]).unwrap();
    assert_eq!(first_order.owner, vec![65, 108, 105, 99, 101]);
    let second_order = storage.get_order(ids[1]).unwrap();
    assert_eq!(second_order.owner, vec![66, 111, 98]);
    let third_order = storage.get_order(ids[2]).unwrap();
    assert_eq!(third_order.owner, vec![67, 97, 114, 111, 108]);
}

#[test]
fn pop_front_removes_head() {
    let mut storage = setup_orders();
    let front = storage.pop_front(100);
    assert!(front.is_some());

    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 2);
}

#[test]
fn decrease_order_removes_when_below_dust() {
    let mut storage = OrderStorage::new();
    let (id, _) = storage
        .create_order(vec![1], vec![65, 108, 105, 99, 101], 100, 75, 75, 0)
        .unwrap();
    storage.insert_id(100, id, 75).unwrap();

    let (sent, deleted_price) = storage.decrease_order(id, 100, 1, false).unwrap();
    assert_eq!(sent, 75);
    assert_eq!(deleted_price, Some(100));
    assert!(storage.is_empty(100));
}

