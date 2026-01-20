use offgrid_primitives::spot::prices::{L2, PriceNode, Level};
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
    // check the bid price head
    assert_eq!(l2.bid_price_head, Some(100));
    // check the bid price tail
    assert_eq!(l2.bid_price_tail, Some(90));
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
    // check the bid price head
    assert_eq!(l2.bid_price_head, Some(100));
    // check the bid price tail
    assert_eq!(l2.bid_price_tail, Some(80));
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
    assert_eq!(l2.ask_price_nodes, BTreeMap::from([
        (100, PriceNode { prev: None, next: Some(110) }),
        (110, PriceNode { prev: Some(100), next: None })
    ]));
    // check the ask price head
    assert_eq!(l2.ask_price_head, Some(100));
    // check the ask price tail
    assert_eq!(l2.ask_price_tail, Some(110));
}


#[test]
fn insert_ask_price_in_middle_of_list() {
    let mut l2 = L2::new();
    l2.insert_price(false, 100).expect("insert ask price 100");
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    assert_eq!(l2.ask_price_nodes, BTreeMap::from([
        (80, PriceNode { prev: None, next: Some(90) }),
        (90, PriceNode { prev: Some(80), next: Some(100) }),
        (100, PriceNode { prev: Some(90), next: None })
    ]));
    // check the ask price head (lowest price for ask prices in ascending order)
    assert_eq!(l2.ask_price_head, Some(80));
    // check the ask price tail (highest price for ask prices in ascending order)
    assert_eq!(l2.ask_price_tail, Some(100));
}

#[test]
fn clear_bid_head_clears_head_price() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");
    l2.insert_price(true, 80).expect("insert bid price 80");
    
    // Verify initial state: head is 100, tail is 80
    assert_eq!(l2.bid_price_head, Some(100));
    assert_eq!(l2.bid_price_tail, Some(80));
    
    // Clear the head (100), should move to next (90)
    let result = l2.clear_head(true);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(90));
    
    // Verify head moved to 90, tail remains 80
    assert_eq!(l2.bid_price_head, Some(90));
    assert_eq!(l2.bid_price_tail, Some(80));
}

#[test]
fn clear_ask_head_clears_head_price() {
    let mut l2 = L2::new();
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    l2.insert_price(false, 100).expect("insert ask price 100");
    
    // Verify initial state: head is 80, tail is 100
    assert_eq!(l2.ask_price_head, Some(80));
    assert_eq!(l2.ask_price_tail, Some(100));
    
    // Clear the head (80), should move to next (90)
    let result = l2.clear_head(false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(90));
    
    // Verify head moved to 90, tail remains 100
    assert_eq!(l2.ask_price_head, Some(90));
    assert_eq!(l2.ask_price_tail, Some(100));
}

#[test]
fn clear_bid_head_clears_last_price() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    
    // Verify initial state: head is 100, tail is 100 (same when only one price)
    assert_eq!(l2.bid_price_head, Some(100));
    assert_eq!(l2.bid_price_tail, Some(100));
    
    // Clear the head (100), should become None since there's no next
    let result = l2.clear_head(true);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
    
    // Verify both head and tail are now None
    assert_eq!(l2.bid_price_head, None);
    assert_eq!(l2.bid_price_tail, None);
}

#[test]
fn clear_ask_head_clears_last_price() {
    let mut l2 = L2::new();
    l2.insert_price(false, 100).expect("insert ask price 100");
    
    // Verify initial state: head is 100, tail is 100 (same when only one price)
    assert_eq!(l2.ask_price_head, Some(100));
    assert_eq!(l2.ask_price_tail, Some(100));
    
    // Clear the head (100), should become None since there's no next
    let result = l2.clear_head(false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
    
    // Verify both head and tail are now None
    assert_eq!(l2.ask_price_head, None);
    assert_eq!(l2.ask_price_tail, None);
}

// remove price tests
#[test]
fn remove_bid_price_head() {
    let mut l2 = L2::new();
    // Insert prices: 100, 90, 80, 70 (descending order)
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");
    l2.insert_price(true, 80).expect("insert bid price 80");
    l2.insert_price(true, 70).expect("insert bid price 70");

    // Verify initial order: [100, 90, 80, 70]
    assert_eq!(l2.collect_bid_prices(), vec![100, 90, 80, 70]);
    assert_eq!(l2.bid_price_head, Some(100));
    assert_eq!(l2.bid_price_tail, Some(70));

    // Remove head (100)
    l2.remove_price(true, 100).expect("remove bid price 100");

    // Verify order is maintained: [90, 80, 70]
    assert_eq!(l2.collect_bid_prices(), vec![90, 80, 70]);
    assert_eq!(l2.bid_price_head, Some(90));
    assert_eq!(l2.bid_price_tail, Some(70));
    assert!(l2.bid_price_nodes.get(&100).is_none());
}

#[test]
fn remove_bid_price_tail() {
    let mut l2 = L2::new();
    // Insert prices: 100, 90, 80, 70
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");
    l2.insert_price(true, 80).expect("insert bid price 80");
    l2.insert_price(true, 70).expect("insert bid price 70");

    // Remove tail (70)
    l2.remove_price(true, 70).expect("remove bid price 70");

    // Verify order is maintained: [100, 90, 80]
    assert_eq!(l2.collect_bid_prices(), vec![100, 90, 80]);
    assert_eq!(l2.bid_price_head, Some(100));
    assert_eq!(l2.bid_price_tail, Some(80));
    assert!(l2.bid_price_nodes.get(&70).is_none());
}

#[test]
fn remove_bid_price_middle() {
    let mut l2 = L2::new();
    // Insert prices: 100, 90, 80, 70
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");
    l2.insert_price(true, 80).expect("insert bid price 80");
    l2.insert_price(true, 70).expect("insert bid price 70");

    // Remove middle (90)
    l2.remove_price(true, 90).expect("remove bid price 90");

    // Verify order is maintained: [100, 80, 70]
    assert_eq!(l2.collect_bid_prices(), vec![100, 80, 70]);
    assert_eq!(l2.bid_price_head, Some(100));
    assert_eq!(l2.bid_price_tail, Some(70));
    assert!(l2.bid_price_nodes.get(&90).is_none());
    
    // Verify links are correct
    let node_100 = l2.bid_price_nodes.get(&100).unwrap();
    assert_eq!(node_100.next, Some(80));
    let node_80 = l2.bid_price_nodes.get(&80).unwrap();
    assert_eq!(node_80.prev, Some(100));
    assert_eq!(node_80.next, Some(70));
}

#[test]
fn remove_bid_price_multiple() {
    let mut l2 = L2::new();
    // Insert prices: 100, 90, 80, 70, 60
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");
    l2.insert_price(true, 80).expect("insert bid price 80");
    l2.insert_price(true, 70).expect("insert bid price 70");
    l2.insert_price(true, 60).expect("insert bid price 60");

    // Remove multiple prices
    l2.remove_price(true, 90).expect("remove bid price 90"); // middle
    assert_eq!(l2.collect_bid_prices(), vec![100, 80, 70, 60]);
    
    l2.remove_price(true, 100).expect("remove bid price 100"); // head
    assert_eq!(l2.collect_bid_prices(), vec![80, 70, 60]);
    
    l2.remove_price(true, 60).expect("remove bid price 60"); // tail
    assert_eq!(l2.collect_bid_prices(), vec![80, 70]);
    
    l2.remove_price(true, 80).expect("remove bid price 80");
    assert_eq!(l2.collect_bid_prices(), vec![70]);
    
    l2.remove_price(true, 70).expect("remove bid price 70");
    assert_eq!(l2.collect_bid_prices(), vec![]);
    assert_eq!(l2.bid_price_head, None);
    assert_eq!(l2.bid_price_tail, None);
}

#[test]
fn remove_bid_price_nonexistent() {
    let mut l2 = L2::new();
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 90).expect("insert bid price 90");

    // Remove non-existent price should not error
    l2.remove_price(true, 50).expect("remove non-existent bid price");
    
    // Verify list is unchanged
    assert_eq!(l2.collect_bid_prices(), vec![100, 90]);
}

#[test]
fn remove_ask_price_head() {
    let mut l2 = L2::new();
    // Insert prices: 70, 80, 90, 100 (ascending order)
    l2.insert_price(false, 70).expect("insert ask price 70");
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    l2.insert_price(false, 100).expect("insert ask price 100");

    // Verify initial order: [70, 80, 90, 100]
    assert_eq!(l2.collect_ask_prices(), vec![70, 80, 90, 100]);
    assert_eq!(l2.ask_price_head, Some(70));
    assert_eq!(l2.ask_price_tail, Some(100));

    // Remove head (70)
    l2.remove_price(false, 70).expect("remove ask price 70");

    // Verify order is maintained: [80, 90, 100]
    assert_eq!(l2.collect_ask_prices(), vec![80, 90, 100]);
    assert_eq!(l2.ask_price_head, Some(80));
    assert_eq!(l2.ask_price_tail, Some(100));
    assert!(l2.ask_price_nodes.get(&70).is_none());
}

#[test]
fn remove_ask_price_tail() {
    let mut l2 = L2::new();
    // Insert prices: 70, 80, 90, 100
    l2.insert_price(false, 70).expect("insert ask price 70");
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    l2.insert_price(false, 100).expect("insert ask price 100");

    // Remove tail (100)
    l2.remove_price(false, 100).expect("remove ask price 100");

    // Verify order is maintained: [70, 80, 90]
    assert_eq!(l2.collect_ask_prices(), vec![70, 80, 90]);
    assert_eq!(l2.ask_price_head, Some(70));
    assert_eq!(l2.ask_price_tail, Some(90));
    assert!(l2.ask_price_nodes.get(&100).is_none());
}

#[test]
fn remove_ask_price_middle() {
    let mut l2 = L2::new();
    // Insert prices: 70, 80, 90, 100
    l2.insert_price(false, 70).expect("insert ask price 70");
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    l2.insert_price(false, 100).expect("insert ask price 100");

    // Remove middle (80)
    l2.remove_price(false, 80).expect("remove ask price 80");

    // Verify order is maintained: [70, 90, 100]
    assert_eq!(l2.collect_ask_prices(), vec![70, 90, 100]);
    assert_eq!(l2.ask_price_head, Some(70));
    assert_eq!(l2.ask_price_tail, Some(100));
    assert!(l2.ask_price_nodes.get(&80).is_none());
    
    // Verify links are correct
    let node_70 = l2.ask_price_nodes.get(&70).unwrap();
    assert_eq!(node_70.next, Some(90));
    let node_90 = l2.ask_price_nodes.get(&90).unwrap();
    assert_eq!(node_90.prev, Some(70));
    assert_eq!(node_90.next, Some(100));
}

#[test]
fn remove_ask_price_multiple() {
    let mut l2 = L2::new();
    // Insert prices: 60, 70, 80, 90, 100
    l2.insert_price(false, 60).expect("insert ask price 60");
    l2.insert_price(false, 70).expect("insert ask price 70");
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 90).expect("insert ask price 90");
    l2.insert_price(false, 100).expect("insert ask price 100");

    // Remove multiple prices
    l2.remove_price(false, 80).expect("remove ask price 80"); // middle
    assert_eq!(l2.collect_ask_prices(), vec![60, 70, 90, 100]);
    
    l2.remove_price(false, 60).expect("remove ask price 60"); // head
    assert_eq!(l2.collect_ask_prices(), vec![70, 90, 100]);
    
    l2.remove_price(false, 100).expect("remove ask price 100"); // tail
    assert_eq!(l2.collect_ask_prices(), vec![70, 90]);
    
    l2.remove_price(false, 70).expect("remove ask price 70");
    assert_eq!(l2.collect_ask_prices(), vec![90]);
    
    l2.remove_price(false, 90).expect("remove ask price 90");
    assert_eq!(l2.collect_ask_prices(), vec![]);
    assert_eq!(l2.ask_price_head, None);
    assert_eq!(l2.ask_price_tail, None);
}

#[test]
fn remove_ask_price_nonexistent() {
    let mut l2 = L2::new();
    l2.insert_price(false, 70).expect("insert ask price 70");
    l2.insert_price(false, 80).expect("insert ask price 80");

    // Remove non-existent price should not error
    l2.remove_price(false, 150).expect("remove non-existent ask price");
    
    // Verify list is unchanged
    assert_eq!(l2.collect_ask_prices(), vec![70, 80]);
}

#[test]
fn remove_single_price() {
    let mut l2 = L2::new();
    
    // Test bid: single price
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.remove_price(true, 100).expect("remove bid price 100");
    assert_eq!(l2.collect_bid_prices(), vec![]);
    assert_eq!(l2.bid_price_head, None);
    assert_eq!(l2.bid_price_tail, None);
    
    // Test ask: single price
    l2.insert_price(false, 100).expect("insert ask price 100");
    l2.remove_price(false, 100).expect("remove ask price 100");
    assert_eq!(l2.collect_ask_prices(), vec![]);
    assert_eq!(l2.ask_price_head, None);
    assert_eq!(l2.ask_price_tail, None);
}

#[test]
fn bid_order_maintained_after_removal() {
    let mut l2 = L2::new();
    // Insert prices in random order
    l2.insert_price(true, 80).expect("insert bid price 80");
    l2.insert_price(true, 100).expect("insert bid price 100");
    l2.insert_price(true, 60).expect("insert bid price 60");
    l2.insert_price(true, 90).expect("insert bid price 90");
    l2.insert_price(true, 70).expect("insert bid price 70");

    // Should be sorted descending: [100, 90, 80, 70, 60]
    assert_eq!(l2.collect_bid_prices(), vec![100, 90, 80, 70, 60]);

    // Remove 80 (middle)
    l2.remove_price(true, 80).expect("remove bid price 80");
    assert_eq!(l2.collect_bid_prices(), vec![100, 90, 70, 60]);

    // Remove 100 (head)
    l2.remove_price(true, 100).expect("remove bid price 100");
    assert_eq!(l2.collect_bid_prices(), vec![90, 70, 60]);

    // Remove 60 (tail)
    l2.remove_price(true, 60).expect("remove bid price 60");
    assert_eq!(l2.collect_bid_prices(), vec![90, 70]);
}

#[test]
fn ask_order_maintained_after_removal() {
    let mut l2 = L2::new();
    // Insert prices in random order
    l2.insert_price(false, 80).expect("insert ask price 80");
    l2.insert_price(false, 100).expect("insert ask price 100");
    l2.insert_price(false, 60).expect("insert ask price 60");
    l2.insert_price(false, 90).expect("insert ask price 90");
    l2.insert_price(false, 70).expect("insert ask price 70");

    // Should be sorted ascending: [60, 70, 80, 90, 100]
    assert_eq!(l2.collect_ask_prices(), vec![60, 70, 80, 90, 100]);

    // Remove 80 (middle)
    l2.remove_price(false, 80).expect("remove ask price 80");
    assert_eq!(l2.collect_ask_prices(), vec![60, 70, 90, 100]);

    // Remove 60 (head)
    l2.remove_price(false, 60).expect("remove ask price 60");
    assert_eq!(l2.collect_ask_prices(), vec![70, 90, 100]);

    // Remove 100 (tail)
    l2.remove_price(false, 100).expect("remove ask price 100");
    assert_eq!(l2.collect_ask_prices(), vec![70, 90]);
}

// snapshot tests
#[test]
fn get_snapshot_bid_levels() {
    let mut l2 = L2::new();
    let scale = 100_000_000; // 1.00000000 in 8 decimals
    
    // Set bid levels
    let levels = vec![
        Level { price: 100_000_000, pqty: 50_000_000 , cqty: 50_000_000  },  // 1.0 price, 0.5 quantity
        Level { price: 99_000_000, pqty: 30_000_000 , cqty: 30_000_000  },  // 0.99 price, 0.3 quantity
        Level { price: 98_000_000, pqty: 20_000_000 , cqty: 20_000_000  },  // 0.98 price, 0.2 quantity
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get snapshot with step = 3
    let snapshot = l2.get_snapshot_raw(true, scale, 3).expect("get bid snapshot");
    
    // Verify format: array of arrays with [price, quantity]
    assert_eq!(snapshot.len(), 3);
    assert_eq!(snapshot[0], vec![100_000_000, 50_000_000, 50_000_000]);
    assert_eq!(snapshot[1], vec![99_000_000, 30_000_000, 30_000_000]);
    assert_eq!(snapshot[2], vec![98_000_000, 20_000_000, 20_000_000]);
}

#[test]
fn get_snapshot_ask_levels() {
    let mut l2 = L2::new();
    let scale = 100_000_000; // 1.00000000 in 8 decimals
    
    // Set ask levels
    let levels = vec![
        Level { price: 101_000_000, pqty: 40_000_000 , cqty: 40_000_000  },  // 1.01 price, 0.4 quantity
        Level { price: 102_000_000, pqty: 60_000_000 , cqty: 60_000_000  },  // 1.02 price, 0.6 quantity
        Level { price: 103_000_000, pqty: 80_000_000 , cqty: 80_000_000  },  // 1.03 price, 0.8 quantity
    ];
    let _ =l2.set_ask_levels(scale, levels);
    
    // Get snapshot with step = 3
    let snapshot = l2.get_snapshot_raw(false, scale, 3).expect("get ask snapshot");
    
    // Verify format: array of arrays with [price, quantity]
    assert_eq!(snapshot.len(), 3);
    assert_eq!(snapshot[0], vec![101_000_000, 40_000_000, 40_000_000]);
    assert_eq!(snapshot[1], vec![102_000_000, 60_000_000, 60_000_000]);
    assert_eq!(snapshot[2], vec![103_000_000, 80_000_000, 80_000_000]);
}

#[test]
fn get_snapshot_step_smaller_than_levels() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set 5 bid levels
    let levels = vec![
        Level { price: 100_000_000, pqty: 10_000_000 , cqty: 10_000_000  },
        Level { price: 99_000_000, pqty: 20_000_000 , cqty: 20_000_000  },
        Level { price: 98_000_000, pqty: 30_000_000 , cqty: 30_000_000  },
        Level { price: 97_000_000, pqty: 40_000_000 , cqty: 40_000_000  },
        Level { price: 96_000_000, pqty: 50_000_000 , cqty: 50_000_000  },
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get snapshot with step = 2 (should only return 2 levels)
    let snapshot = l2.get_snapshot_raw(true, scale, 2).expect("get bid snapshot");
    
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0], vec![100_000_000, 10_000_000, 10_000_000]);
    assert_eq!(snapshot[1], vec![99_000_000, 20_000_000, 20_000_000]);
}

#[test]
fn get_snapshot_step_larger_than_levels() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set only 2 bid levels
    let levels = vec![
        Level { price: 100_000_000, pqty: 10_000_000 , cqty: 10_000_000  },
        Level { price: 99_000_000, pqty: 20_000_000 , cqty: 20_000_000  },
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get snapshot with step = 5 (should only return 2 levels that exist)
    let snapshot = l2.get_snapshot_raw(true, scale, 5).expect("get bid snapshot");
    
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0], vec![100_000_000, 10_000_000, 10_000_000]);
    assert_eq!(snapshot[1], vec![99_000_000, 20_000_000, 20_000_000]);
}

#[test]
fn get_snapshot_empty_levels() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set empty levels
    let _ = l2.set_bid_levels(scale, vec![]);
    
    // Get snapshot should return empty array
    let snapshot = l2.get_snapshot_raw(true, scale, 5).expect("get bid snapshot");
    
    assert_eq!(snapshot.len(), 0);
}

#[test]
fn get_snapshot_nonexistent_scale() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    let nonexistent_scale = 200_000_000;
    
    // Set levels for one scale
    let levels = vec![
        Level { price: 100_000_000, pqty: 10_000_000 , cqty: 10_000_000  },
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get snapshot for nonexistent scale should return empty array
    let snapshot = l2.get_snapshot_raw(true, nonexistent_scale, 5).expect("get bid snapshot");
    
    assert_eq!(snapshot.len(), 0);
}

#[test]
fn get_snapshot_multiple_scales() {
    let mut l2 = L2::new();
    let scale1 = 100_000_000; // 1.0
    let scale2 = 1_000_000_000; // 10.0
    
    // Set bid levels for scale1
    let levels1 = vec![
        Level { price: 100_000_000, pqty: 10_000_000 , cqty: 10_000_000  },
        Level { price: 99_000_000, pqty: 20_000_000 , cqty: 20_000_000  },
    ];
    let _ = l2.set_bid_levels(scale1, levels1);
    
    // Set bid levels for scale2
    let levels2 = vec![
        Level { price: 1_000_000_000, pqty: 100_000_000 , cqty: 100_000_000  },
        Level { price: 990_000_000, pqty: 200_000_000 , cqty: 200_000_000  },
    ];
    let _ = l2.set_bid_levels(scale2, levels2);
    
    // Get snapshot for scale1
    let snapshot1 = l2.get_snapshot_raw(true, scale1, 5).expect("get bid snapshot scale1");
    assert_eq!(snapshot1.len(), 2);
    assert_eq!(snapshot1[0], vec![100_000_000, 10_000_000, 10_000_000]);
    assert_eq!(snapshot1[1], vec![99_000_000, 20_000_000, 20_000_000]);
    
    // Get snapshot for scale2
    let snapshot2 = l2.get_snapshot_raw(true, scale2, 5).expect("get bid snapshot scale2");
    assert_eq!(snapshot2.len(), 2);
    assert_eq!(snapshot2[0], vec![1_000_000_000, 100_000_000, 100_000_000]);
    assert_eq!(snapshot2[1], vec![990_000_000, 200_000_000, 200_000_000]);
}

#[test]
fn get_snapshot_bid_vs_ask_separation() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set bid levels
    let bid_levels = vec![
        Level { price: 100_000_000, pqty: 10_000_000 , cqty: 10_000_000  },
        Level { price: 99_000_000, pqty: 20_000_000 , cqty: 20_000_000  },
    ];
    let _ = l2.set_bid_levels(scale, bid_levels);
    
    // Set ask levels
    let ask_levels = vec![
        Level { price: 101_000_000, pqty: 30_000_000 , cqty: 30_000_000  },
        Level { price: 102_000_000, pqty: 40_000_000 , cqty: 40_000_000  },
    ];
    let _ = l2.set_ask_levels(scale, ask_levels);
    
    // Get bid snapshot
    let bid_snapshot = l2.get_snapshot_raw(true, scale, 5).expect("get bid snapshot");
    assert_eq!(bid_snapshot.len(), 2);
    assert_eq!(bid_snapshot[0], vec![100_000_000, 10_000_000, 10_000_000]);
    assert_eq!(bid_snapshot[1], vec![99_000_000, 20_000_000, 20_000_000]);
    
    // Get ask snapshot
    let ask_snapshot = l2.get_snapshot_raw(false, scale, 5).expect("get ask snapshot");
    assert_eq!(ask_snapshot.len(), 2);
    assert_eq!(ask_snapshot[0], vec![101_000_000, 30_000_000, 30_000_000]);
    assert_eq!(ask_snapshot[1], vec![102_000_000, 40_000_000, 40_000_000]);
}

#[test]
fn get_snapshot_large_quantities() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set levels with large quantities (but still fit in u64)
    let levels = vec![
        Level { price: 100_000_000, pqty: 1_000_000_000_000_000_000 , cqty: 1_000_000_000_000_000_000  },  // Large quantity
        Level { price: 99_000_000, pqty: 500_000_000_000_000_000 , cqty: 500_000_000_000_000_000  },
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get snapshot
    let snapshot = l2.get_snapshot_raw(true, scale, 2).expect("get bid snapshot");
    
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0], vec![100_000_000, 1_000_000_000_000_000_000, 1_000_000_000_000_000_000]);
    assert_eq!(snapshot[1], vec![99_000_000, 500_000_000_000_000_000, 500_000_000_000_000_000]);
}

#[test]
fn get_snapshot_zero_step() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set levels
    let levels = vec![
        Level { price: 100_000_000, pqty: 10_000_000 , cqty: 10_000_000  },
        Level { price: 99_000_000, pqty: 20_000_000 , cqty: 20_000_000  },
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get snapshot with step = 0 (should return empty array)
    let snapshot = l2.get_snapshot_raw(true, scale, 0).expect("get bid snapshot");
    
    assert_eq!(snapshot.len(), 0);
}

#[test]
fn get_snapshot_formatted_strings() {
    let mut l2 = L2::new();
    let scale = 100_000_000; // 1.00000000 in 8 decimals
    
    // Set bid levels
    let levels = vec![
        Level { price: 100_000_000, pqty: 50_000_000 , cqty: 50_000_000  },  // 1.0 price, 0.5 quantity
        Level { price: 99_000_000, pqty: 30_000_000 , cqty: 30_000_000  },  // 0.99 price, 0.3 quantity
        Level { price: 98_000_000, pqty: 20_000_000 , cqty: 20_000_000  },  // 0.98 price, 0.2 quantity
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get formatted snapshot
    let snapshot = l2.get_snapshot(true, scale, 3).expect("get formatted bid snapshot");
    
    // Verify format: array of arrays with formatted strings
    assert_eq!(snapshot.len(), 3);
    assert_eq!(snapshot[0], vec!["1.00000000".to_string(), "0.50000000".to_string(), "0.50000000".to_string()]);
    assert_eq!(snapshot[1], vec!["0.99000000".to_string(), "0.30000000".to_string(), "0.30000000".to_string()]);
    assert_eq!(snapshot[2], vec!["0.98000000".to_string(), "0.20000000".to_string(), "0.20000000".to_string()]);
}

#[test]
fn get_snapshot_formatted_strings_edge_cases() {
    let mut l2 = L2::new();
    let scale = 100_000_000;
    
    // Set levels with various edge cases
    let levels = vec![
        Level { price: 0, pqty: 0 , cqty: 0  },                    // zero values
        Level { price: 1, pqty: 1 , cqty: 1  },                     // very small values
        Level { price: 1_000_000_000, pqty: 500_000_000 , cqty: 500_000_000  }, // 10.0 price, 5.0 quantity
    ];
    let _ = l2.set_bid_levels(scale, levels);
    
    // Get formatted snapshot
    let snapshot = l2.get_snapshot(true, scale, 3).expect("get formatted bid snapshot");
    
    assert_eq!(snapshot.len(), 3);
    assert_eq!(snapshot[0], vec!["0.00000000".to_string(), "0.00000000".to_string(), "0.00000000".to_string()]);
    assert_eq!(snapshot[1], vec!["0.00000001".to_string(), "0.00000001".to_string(), "0.00000001".to_string()]);
    assert_eq!(snapshot[2], vec!["10.00000000".to_string(), "5.00000000".to_string(), "5.00000000".to_string()]);
}
