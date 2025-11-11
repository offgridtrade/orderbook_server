use super::{Order, OrderStorage};
use crate::primitives::{save_levels_snapshot, L1, L2, Level};
use rust_rocksdb::DB;
use tempfile::tempdir;

#[test]
fn serialize_and_deserialize_storage() {
    let mut storage = OrderStorage::new();
    let (id, _) = storage
        .create_order(vec![1], vec![2], 100, 10, 10, 0)
        .expect("create order");
    storage.insert_id(100, id, 10).expect("insert id");

    let encoded = bincode::serialize(&storage).expect("serialize storage");
    let decoded: OrderStorage = bincode::deserialize(&encoded).expect("deserialize storage");

    assert_eq!(decoded.get_order(id), storage.get_order(id));
}

#[test]
fn serialize_and_deserialize_order() {
    let order = Order::new(vec![1], vec![2], 100, 10, 10, 10, 0);
    let encoded = bincode::serialize(&order).expect("serialize order");
    let decoded: Order = bincode::deserialize(&encoded).expect("deserialize order");
    assert_eq!(decoded, order);
}

#[test]
// save data in storage to snapshot on rocksdb, and get it back. compare the data is the same.
fn save_and_load_storage() {
    let mut storage = OrderStorage::new();
    let (id, _) = storage
        .create_order(vec![1], vec![2], 100, 10, 10, 0)
        .expect("create order");
    storage.insert_id(100, id, 10).expect("insert id");

    let l1 = L1::new(100, 90, 110, 5, 5, 10, 10);
    let l2 = L2 {
        bids: vec![Level {
            price: 100,
            quantity: 1_000,
        }],
        asks: vec![Level {
            price: 110,
            quantity: 2_000,
        }],
        ..Default::default()
    };

    let dir = tempdir().expect("create temp dir");
    save_levels_snapshot(dir.path(), &l1, &l2, &storage).expect("save snapshot");

    let db = DB::open_default(dir.path()).expect("open db");
    let loaded_storage: OrderStorage = bincode::deserialize(&db.get(b"snapshot:l3").unwrap().unwrap())
        .expect("deserialize l3");

    assert_eq!(loaded_storage.get_order(id), storage.get_order(id));
}