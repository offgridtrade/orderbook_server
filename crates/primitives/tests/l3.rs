use offgrid_primitives::{orders::{L3, Order, L3Error}};

fn setup_orders() -> L3 {
    let mut storage = L3::new();
    let (id1, _) = storage
        .create_order("1", "alice", 100, 50, 50, 0)
        .expect("create order 1");
    storage.insert_id(100, id1, 50).expect("insert order 1");

    let (id2, _) = storage
        .create_order("2", "bob", 100, 75, 75, 0)
        .expect("create order 2");
    storage.insert_id(100, id2, 75).expect("insert order 2");

    let (id3, _) = storage
        .create_order("3", "carol", 100, 20, 20, 0)
        .expect("create order 3");
    storage.insert_id(100, id3, 20).expect("insert order 3");

    storage
}

fn setup_orders_with_price_level_shift_scenario() -> L3 {
    let mut storage = L3::new();
    let price = 100u64;
    let (id1, _) = storage
        .create_order("1", "alice", price, 50, 50, 0)
        .expect("create order 1");
    storage.insert_id(price, id1, 50).expect("insert order 1");
    storage
}

fn sample_order() -> Order {
    Order::new(vec![1], vec![2], 100, 10, 10, 10, 0)
}

#[test]
fn inserts_orders_fifo() {
    let storage = setup_orders();
    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 3);

    // check if the order ids are in the correct order for FIFO, [1, 2, 3]
    assert_eq!(ids, vec![1, 2, 3]);
    let first_order = storage.get_order(ids[0]).unwrap();
    assert_eq!(first_order.owner, vec![65, 108, 105, 99, 101]);
    let second_order = storage.get_order(ids[1]).unwrap();
    assert_eq!(second_order.owner, vec![66, 111, 98]);
    let third_order = storage.get_order(ids[2]).unwrap();
    assert_eq!(third_order.owner, vec![67, 97, 114, 111, 108]);
}

#[test]
fn pop_front_removes_first_order_within_given_price_level() {
    let mut storage = setup_orders();
    let front = storage.pop_front(100);
    assert!(front.is_ok());
    assert_eq!(front.unwrap(), (Some(1), false));
}

#[test]
fn pop_front_removes_head() {
    // check if the head and tail are set correctly
    let mut storage = setup_orders_with_price_level_shift_scenario();
    assert!(storage.head(100) == Some(1));
    assert!(storage.tail(100) == Some(1));
    // pop the first front order
    let front = storage.pop_front(100);
    assert!(front.is_ok());
    assert_eq!(front.unwrap(), (Some(1), true));
    // check if the head and tail are removed
    assert!(storage.head(100).is_none());
    assert!(storage.tail(100).is_none());

    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 0);
}

#[test]
fn delete_order_removes_order_from_storage_and_returns_none_if_order_does_not_exist() {
    let mut storage = setup_orders_with_price_level_shift_scenario();
    let result = storage.delete_order(10);
    assert!(result.is_err());
    assert!(matches!(result, Err(L3Error::OrderDoesNotExist(10))));
}

#[test]
fn delete_order_removes_order_from_storage_and_return_emptied_price_level() {
    let mut storage = setup_orders_with_price_level_shift_scenario();
    let result = storage.delete_order(1);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(100));
    assert!(storage.is_empty(100));

    assert!(storage.head(100).is_none());
    assert!(storage.tail(100).is_none());
    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 0);
}

#[test]
fn delete_order_removes_order_from_storage_in_the_middle_of_the_price_level() {
    let mut storage = setup_orders();
    let result = storage.delete_order(2);
    assert!(result.is_ok());
    // should return none as the price level is not empty
    assert_eq!(result.unwrap(), None);
    assert!(!storage.is_empty(100));
    // head and tail should be same from before and after the deletion
    assert_eq!(storage.head(100), Some(1));
    assert_eq!(storage.tail(100), Some(3));
    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids, vec![1, 3]);
    // check if the orders are in the correct order for FIFO, [1, 3]
    let first_order = storage.get_order(1).unwrap();
    assert_eq!(first_order.owner, "alice".as_bytes());
    let second_order = storage.get_order(3).unwrap();
    assert_eq!(second_order.owner, "carol".as_bytes());
}

#[test]
fn delete_order_removes_order_from_storage_end() {
    let mut storage = setup_orders();
    let result = storage.delete_order(3);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
    assert!(!storage.is_empty(100));
    // check ids
    let ids = storage.get_order_ids(100, 3);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids, vec![1, 2]);
    // head should be 1
    assert_eq!(storage.head(100), Some(1));
    // tail should be 2
    assert_eq!(storage.tail(100), Some(2));
}


#[test]
fn decrease_order_removes_when_below_dust() {
    let mut storage = L3::new();
    let (id, _) = storage
        .create_order("1", "alice", 100, 75, 75, 0)
        .expect("create order");
    storage.insert_id(100, id, 75).expect("insert id");

    let (sent, deleted_price) = storage.decrease_order(id, 100, 1, false).unwrap();
    assert_eq!(sent, 75);
    assert_eq!(deleted_price, Some(100));
    assert!(storage.is_empty(100));
}

#[test]
fn serialize_and_deserialize_storage() {
    let storage = setup_orders();
    let encoded = bincode::serialize(&storage).expect("serialize storage");
    let decoded: L3 = bincode::deserialize(&encoded).expect("deserialize storage");

    assert_eq!(
        decoded.get_order_ids(100, 3),
        storage.get_order_ids(100, 3)
    );
}

#[test]
fn serialize_and_deserialize_order() {
    let order = sample_order();
    let encoded = bincode::serialize(&order).expect("serialize order");
    let decoded: Order = bincode::deserialize(&encoded).expect("deserialize order");
    assert_eq!(decoded, order);
}

#[test]
fn ensure_price_zero_is_error() {
    let mut storage = L3::new();
    let result = storage.insert_id(0, 1, 0);
    assert!(matches!(result, Err(L3Error::PriceIsZero)));
}

