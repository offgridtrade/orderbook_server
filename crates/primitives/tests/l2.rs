use offgrid_primitives::prices::{L2, PriceNode};
use std::collections::BTreeMap;

// price linked list tests
#[test]
fn insert_bid_price() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    assert_eq!(l2.bid_price_head, Some(100));
}

// inserting bid price with nothing places the price at bid head
#[test]
fn insert_bid_price_with_nothing() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    assert_eq!(l2.bid_price_head, Some(100));
}

// inserting bid price with something places the price at descending order
#[test]
fn insert_bid_price_in_descending_order() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");
    // check if the bid price is in descending order
    assert_eq!(l2.bid_price_nodes, BTreeMap::from([(100, PriceNode { prev: None, next: Some(90) }), (90, PriceNode { prev: Some(100), next: None })]));
}

// inserting bid price with something places the price at descending order
// the price gets inserted in the middle of the list
#[test]
fn insert_bid_price_in_middle_of_list() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 80).expect("insert bid price 80");
    l2.insert_price(true, 90).expect("insert bid price 90");
    // check if the bid price is in descending order
    assert_eq!(l2.bid_price_nodes, BTreeMap::from([
        (100, PriceNode { prev: None, next: Some(90) }), 
        (90, PriceNode { prev: Some(100), next: Some(80) }),
        (80, PriceNode { prev: Some(90), next: None })
    ]));
}

// ask price linked list tests
#[test]
fn insert_ask_price() {
    let mut l2 = L2::new();
    l2.insert_price(false, 100).expect("insert ask price 100");
    assert_eq!(l2.ask_price_head, Some(100));
}

#[test]
fn insert_ask_price_with_nothing() {
    let mut l2 = L2::new();
    l2.insert_price(false, 100).expect("insert ask price 100");
    assert_eq!(l2.ask_price_head, Some(100));
}

#[test]
fn insert_ask_price_in_ascending_order() {
    let mut l2 = L2::new();
    l2.insert_price(false, 100).expect("insert ask price 100");
    l2.insert_price(false, 110).expect("insert ask price 110");
    assert_eq!(l2.ask_price_nodes, BTreeMap::from([(100, PriceNode { prev: None, next: Some(110) })]));
}


#[test]
fn insert_ask_price_in_middle_of_list() {
    let mut l2 = L2::new();
    l2.insert_price(false, 100).expect("insert ask price 100");
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    assert_eq!(l2.ask_price_nodes, BTreeMap::from([(80, PriceNode { prev: None, next: Some(90) }), (90, PriceNode { prev: Some(80), next: Some(100) })]));
}


