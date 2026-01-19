use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Level {
    /// price in 8 decimals
    pub price: u64,
    /// public quantity in 8 decimals
    pub pqty: u64,
    /// current quantity in 8 decimals
    pub cqty: u64,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Level(price: {}, pqty: {}, cqty: {})", self.price, self.pqty, self.cqty)
    }
}

// Helper to format Vec<Level> for error messages
fn format_levels(levels: &[Level]) -> String {
    if levels.is_empty() {
        return "[]".to_string();
    }
    let formatted: Vec<String> = levels.iter()
        .map(|level| format!("{}", level))
        .collect();
    format!("[{}]", formatted.join(", "))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PriceNode {
    pub prev: Option<u64>,
    pub next: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct L2 {
    /// Head of the bid price linked list
    pub bid_price_head: Option<u64>,
    /// Head of the ask price linked list
    pub ask_price_head: Option<u64>,
    /// Tail of the bid price linked list
    pub bid_price_tail: Option<u64>,
    /// Tail of the ask price linked list
    pub ask_price_tail: Option<u64>,
    /// Mapping price -> node of the bid price linked list
    pub bid_price_nodes: BTreeMap<u64, PriceNode>,
    /// Mapping price -> node of the ask price linked list
    pub ask_price_nodes: BTreeMap<u64, PriceNode>,
    /// Public bid levels sorted by price descending
    pub public_bid_level_map: BTreeMap<u64, u64>,
    /// Public ask levels sorted by price ascending
    pub public_ask_level_map: BTreeMap<u64, u64>,
    /// Current bid levels sorted by price descending
    pub current_bid_level_map: BTreeMap<u64, u64>,
    /// Current ask levels sorted by price ascending
    pub current_ask_level_map: BTreeMap<u64, u64>,
    /// Bid levels sorted by price descending for snapshot display
    /// key is scale in 8 decimals integer (e.g. 100000000 for 1.00000000, 1000000000 for 10.00000000)
    /// value is a vector of levels in the quantized price space
    pub bid_level_list: BTreeMap<u64, Vec<Level>>,
    /// Ask levels sorted by price ascending for snapshot display
    /// key is scale in 8 decimals integer (e.g. 100000000 for 1.00000000, 1000000000 for 10.00000000)
    /// value is a vector of levels in the quantized price space
    pub ask_level_list: BTreeMap<u64, Vec<Level>>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum L2Error {
    #[error("price is zero in L2 orderbook level")]
    PriceIsZero,
    #[error("price is missing in L2 orderbook level: {price} isBid: {is_bid} isPlaced: {is_placed}")]
    PriceMissing { price: u64, is_bid: bool, is_placed: bool },
    #[error("failed to set bid level: {price} {level}")]
    FailedToSetBidLevel { price: u64, level: u64 },
    #[error("failed to set ask level: {price} {level}")]
    FailedToSetAskLevel { price: u64, level: u64 },
    #[error("failed to set bid levels: scale={scale}, levels={}", format_levels(&levels))]
    FailedToSetBidLevels { scale: u64, levels: Vec<Level> },
    #[error("failed to set ask levels: scale={scale}, levels={}", format_levels(&levels))]
    FailedToSetAskLevels { scale: u64, levels: Vec<Level> },
}

impl L2 {
    pub fn new() -> Self {
        Self {
            bid_price_head: None,
            bid_price_tail: None,
            ask_price_head: None,
            ask_price_tail: None,
            bid_price_nodes: BTreeMap::new(),
            ask_price_nodes: BTreeMap::new(),
            public_bid_level_map: BTreeMap::new(),
            public_ask_level_map: BTreeMap::new(),
            current_bid_level_map: BTreeMap::new(),
            current_ask_level_map: BTreeMap::new(),
            bid_level_list: BTreeMap::new(),
            ask_level_list: BTreeMap::new(),
        }
    }

    fn ensure_price(price: u64) -> Result<(), L2Error> {
        if price == 0 {
            Err(L2Error::PriceIsZero)
        } else {
            Ok(())
        }
    }

    pub fn bid_head(&self) -> Option<u64> {
        self.bid_price_head
    }

    pub fn ask_head(&self) -> Option<u64> {
        self.ask_price_head
    }

    pub fn public_bid_level(&self, price: u64) -> Option<u64> {
        self.public_bid_level_map.get(&price).copied()
    }

    pub fn public_ask_level(&self, price: u64) -> Option<u64> {
        self.public_ask_level_map.get(&price).copied()
    }

    pub fn current_bid_level(&self, price: u64) -> Option<u64> {
        self.current_bid_level_map.get(&price).copied()
    }

    pub fn current_ask_level(&self, price: u64) -> Option<u64> {
        self.current_ask_level_map.get(&price).copied()
    }

    pub fn bid_levels(&self, scale: u64) -> &Vec<Level> {
        &self.bid_level_list.get(&scale).unwrap()
    }

    pub fn ask_levels(&self, scale: u64) -> &Vec<Level> {
        &self.ask_level_list.get(&scale).unwrap()
    }

    pub fn scale_bid_levels(&self, scale: u64, n: u32) -> Vec<Level> {
        let levels = self.bid_level_list.get(&scale).cloned().unwrap_or(Vec::new());
        levels.iter().take(n as usize).cloned().collect()
    }

    pub fn scale_ask_levels(&self, scale: u64, n: u32) -> Vec<Level> {
        let levels = self.ask_level_list.get(&scale).cloned().unwrap_or(Vec::new());
        levels.iter().take(n as usize).cloned().collect()
    }

    pub fn set_current_bid_level(&mut self, price: u64, level: u64) -> Result<(), L2Error> {
        self.current_bid_level_map.insert(price, level);
        Ok(())
    }

    pub fn set_current_ask_level(&mut self, price: u64, level: u64) -> Result<(), L2Error> {
        self.current_ask_level_map.insert(price, level);
        Ok(())
    }

    pub fn set_public_bid_level(&mut self, price: u64, level: u64) -> Result<(), L2Error> {
        self.public_bid_level_map.insert(price, level);
        Ok(())
    }

    pub fn set_public_ask_level(&mut self, price: u64, level: u64) -> Result<(), L2Error> {
        self.public_ask_level_map.insert(price, level);
        Ok(())
    }

    pub fn set_bid_levels(&mut self, scale: u64, levels: Vec<Level>) -> Result<(), L2Error> {
        self.bid_level_list.insert(scale, levels);
        Ok(())
    }

    pub fn set_ask_levels(&mut self, scale: u64, levels: Vec<Level>) -> Result<(), L2Error> {
        self.ask_level_list.insert(scale, levels);
        Ok(())
    }

    pub fn clear_head(&mut self, is_bid: bool) -> Result<Option<u64>, L2Error> {
        if is_bid {
            let old_head = self.bid_price_head.unwrap();
            // update the head of the bid price linked list
            // if the next is empty, set the head and tail to none
            if self.bid_price_nodes.get(&old_head).and_then(|node| node.next).is_none() {
                self.bid_price_head = None;
                self.bid_price_tail = None;
            }
            else {
                self.bid_price_head = self.bid_price_nodes.get(&old_head).and_then(|node| node.next);
            }
            // remove the node from the bid price nodes map
            self.bid_price_nodes.remove(&old_head);
            Ok(self.bid_price_head)
        } else {
            let old_head = self.ask_price_head.unwrap();
            // update the head of the ask price linked list
            // if the next is empty, set the head and tail to none
            if self.ask_price_nodes.get(&old_head).and_then(|node| node.next).is_none() {
                self.ask_price_head = None;
                self.ask_price_tail = None;
            }
            else {
                self.ask_price_head = self.ask_price_nodes.get(&old_head).and_then(|node| node.next);
            }
            // remove the node from the ask price nodes map
            self.ask_price_nodes.remove(&old_head);
            Ok(self.ask_price_head)
        }
    }

    pub fn price_exists(&self, is_bid: bool, price: u64) -> bool {
        if is_bid {
            self.bid_price_nodes.contains_key(&price)
        }
        else {
            self.ask_price_nodes.contains_key(&price)
        }
    }

    pub fn insert_price(&mut self, is_bid: bool, price: u64) -> Result<(), L2Error> {
        if is_bid {
            let _ = self._insert_bid_price(price)?;
            self.set_public_bid_level(price, 0)?;
            self.set_current_bid_level(price, 0)?;
            Ok(())
        }
        else {
            let _ = self._insert_ask_price(price)?;
            self.set_public_ask_level(price, 0)?;
            self.set_current_ask_level(price, 0)?;
            Ok(())
        }
    }

    /// inserts a bid price into the bid price linked list
    /// price linked list is sorted in descending order
    fn _insert_bid_price(&mut self, price: u64) -> Result<(), L2Error> {
        Self::ensure_price(price)?;
        // compare head of the bid price head
        if self.bid_price_head == None {
            self.bid_price_head = Some(price);
            // set the tail of the bid price linked list
            self.bid_price_tail = Some(price);
            // create a node for the first price
            self.bid_price_nodes.insert(price, PriceNode {
                prev: None,
                next: None,
            });
            return Ok(());
        }
        else if price > self.bid_price_head.unwrap() {
            let old_head = self.bid_price_head.unwrap();
            self.bid_price_head = Some(price);
            // update old head's node to point back to new head
            self.bid_price_nodes.get_mut(&old_head).map(|node| node.prev = Some(price));
            // create node for new head
            self.bid_price_nodes.insert(price, PriceNode {
                prev: None,
                next: Some(old_head),
            });
            return Ok(());
        } 
        else if price == self.bid_price_head.unwrap() {
            return Ok(());
        }
        else {
            // traverse through the bid price linked list and insert the price at the correct position so that the list is sorted in descending order
            let mut current = self.bid_price_head;
            while current.is_some() {
                let next = self.bid_price_nodes.get(&current.unwrap()).and_then(|node| node.next);
                // next does not exist
                if next.is_none() {
                    // insert the price at the current position
                    let curr = current.unwrap();
                    self.bid_price_nodes.get_mut(&curr).map(|node| node.next = Some(price));
                    // create a node for the new price
                    self.bid_price_nodes.insert(price, PriceNode {
                        prev: Some(curr),
                        next: None,
                    });
                    // set the tail of the bid price linked list
                    self.bid_price_tail = Some(price);
                    return Ok(());
                }
                else {
                    // next exists
                    let next_val = next.unwrap();
                    if next_val > price {
                        // traverse until the price is bigger than the current price
                        current = Some(next_val);
                    }
                    else if next_val < price {
                        // To avoid mutable and immutable borrow at the same time, collect value first
                        let curr = current.unwrap();
                        // Update current's next to point to new price
                        self.bid_price_nodes.get_mut(&curr).map(|node| node.next = Some(price));
                        // Update next_val's prev to point to new price
                        self.bid_price_nodes.get_mut(&next_val).map(|node| node.prev = Some(price));
                        // Create node for new price
                        self.bid_price_nodes.insert(price, PriceNode {
                            prev: Some(curr),
                            next: Some(next_val),
                        });
                        return Ok(());
                    }
                }
            }
            return Ok(());
        }
    }

    /// price linked list is sorted in ascending order
    /// linked list is sorted in ascending order
    fn _insert_ask_price(&mut self, price: u64) -> Result<(), L2Error> {
        Self::ensure_price(price)?;
        // compare head of the ask price head
        if self.ask_price_head == None {
            self.ask_price_head = Some(price);
            // set the tail of the ask price linked list
            self.ask_price_tail = Some(price);
            // create a node for the first price
            self.ask_price_nodes.insert(price, PriceNode {
                prev: None,
                next: None,
            });
            return Ok(());
        }
        else if price < self.ask_price_head.unwrap() {
            let old_head = self.ask_price_head.unwrap();
            self.ask_price_head = Some(price);
            // update old head's prev pointer to point to new head
            self.ask_price_nodes.get_mut(&old_head).map(|node| node.prev = Some(price));
            // create node for new head
            self.ask_price_nodes.insert(price, PriceNode {
                prev: None,
                next: Some(old_head),
            });
            return Ok(());
        }
        else if price == self.ask_price_head.unwrap() {
            return Ok(());
        }
        else {
            // traverse through the ask price linked list and insert the price at the correct position so that the list is sorted in ascending order
            let mut current = self.ask_price_head;
            while current.is_some() {
                let next = self.ask_price_nodes.get(&current.unwrap()).and_then(|node| node.next);
                // next does not exist
                if next.is_none() {
                    // insert the price at the tail position
                    let curr = current.unwrap();
                    // update current node's next pointer
                    self.ask_price_nodes.get_mut(&curr).map(|node| node.next = Some(price));
                    // create a node for the new price
                    self.ask_price_nodes.insert(price, PriceNode {
                        prev: Some(curr),
                        next: None,
                    });
                    // set the tail of the ask price linked list
                    self.ask_price_tail = Some(price);
                    return Ok(());
                }
                else {
                    // next exists
                    let next_val = next.unwrap();
                    if next_val < price {
                        // traverse until the price is smaller than the current price
                        current = Some(next_val);
                    }
                    else if next_val > price {
                        // To avoid mutable and immutable borrow at the same time, collect value first
                        let curr = current.unwrap();
                        // Update current's next pointer to point to new price
                        self.ask_price_nodes.get_mut(&curr).map(|node| node.next = Some(price));
                        // Update next_val's prev pointer to point to new price
                        self.ask_price_nodes.get_mut(&next_val).map(|node| node.prev = Some(price));
                        // Create node for new price
                        self.ask_price_nodes.insert(price, PriceNode {
                            prev: Some(curr),
                            next: Some(next_val),
                        });
                        return Ok(());
                    }
                    else {
                        break;
                    }
                }
            }
            return Ok(());
        }
    }

    // remove price from the price linked list
    pub fn remove_price(&mut self, is_bid: bool, price: u64) -> Result<(), L2Error> {
        if is_bid {
            self._remove_bid_price(price)?;
            // remove the level from the level map
            self.public_bid_level_map.remove(&price);
            self.current_bid_level_map.remove(&price);
            Ok(())
        }
        else {
            self._remove_ask_price(price)?;
            // remove the level from the level map
            self.public_ask_level_map.remove(&price);
            self.current_ask_level_map.remove(&price);
            Ok(())
        }
    }
    // remove price from the bid price linked list
    fn _remove_bid_price(&mut self, price: u64) -> Result<(), L2Error> {
        // Get the node to be removed before removing it
        let node = match self.bid_price_nodes.get(&price) {
            Some(node) => node.clone(),
            None => return Ok(()), // Price doesn't exist, nothing to remove
        };

        // Update the previous node's next pointer
        if let Some(prev_price) = node.prev {
            if let Some(prev_node) = self.bid_price_nodes.get_mut(&prev_price) {
                prev_node.next = node.next;
            }
        } else {
            // This is the head, update head pointer
            self.bid_price_head = node.next;
        }

        // Update the next node's prev pointer
        if let Some(next_price) = node.next {
            if let Some(next_node) = self.bid_price_nodes.get_mut(&next_price) {
                next_node.prev = node.prev;
            }
        } else {
            // This is the tail, update tail pointer
            self.bid_price_tail = node.prev;
        }

        // Remove the node from the map
        self.bid_price_nodes.remove(&price);

        // Remove the level from the level map
        self.public_bid_level_map.remove(&price);
        self.current_bid_level_map.remove(&price);

        Ok(())
    }

    // remove price from the ask price linked list
    fn _remove_ask_price(&mut self, price: u64) -> Result<(), L2Error> {
        // Get the node to be removed before removing it
        let node = match self.ask_price_nodes.get(&price) {
            Some(node) => node.clone(),
            None => return Ok(()), // Price doesn't exist, nothing to remove
        };

        // Update the previous node's next pointer
        if let Some(prev_price) = node.prev {
            if let Some(prev_node) = self.ask_price_nodes.get_mut(&prev_price) {
                prev_node.next = node.next;
            }
        } else {
            // This is the head, update head pointer
            self.ask_price_head = node.next;
        }

        // Update the next node's prev pointer
        if let Some(next_price) = node.next {
            if let Some(next_node) = self.ask_price_nodes.get_mut(&next_price) {
                next_node.prev = node.prev;
            }
        } else {
            // This is the tail, update tail pointer
            self.ask_price_tail = node.prev;
        }

        // Remove the node from the map
        self.ask_price_nodes.remove(&price);

        // Remove the level from the level map
        self.public_ask_level_map.remove(&price);
        self.current_ask_level_map.remove(&price);

        Ok(())
    }

    /// Helper function to collect all bid prices in order (descending)
    pub fn collect_bid_prices(&self) -> Vec<u64> {
        let mut prices = Vec::new();
        let mut current = self.bid_price_head;
        while let Some(price) = current {
            prices.push(price);
            current = self.bid_price_nodes.get(&price).and_then(|node| node.next);
        }
        prices
    }

    /// Helper function to collect all ask prices in order (ascending)
    pub fn collect_ask_prices(&self) -> Vec<u64> {
        let mut prices = Vec::new();
        let mut current = self.ask_price_head;
        while let Some(price) = current {
            prices.push(price);
            current = self.ask_price_nodes.get(&price).and_then(|node| node.next);
        }
        prices
    }

    /// Helper function to format a u64 number (in 8 decimals) to a string with 8 decimal places
    /// Example: 100_000_000 -> "1.00000000", 50_000_000 -> "0.50000000"
    fn format_8_decimals(value: u64) -> String {
        const DECIMAL_PLACES: u64 = 100_000_000; // 10^8
        
        let integer_part = value / DECIMAL_PLACES;
        let decimal_part = value % DECIMAL_PLACES;
        
        format!("{}.{:08}", integer_part, decimal_part)
    }

    /// get L2 snapshot (raw numbers)
    /// Returns an array of arrays where each inner array is [price in 8 decimals, base amount in 8 decimals]
    /// The outer array has step length
    pub fn get_snapshot_raw(&self, is_bid: bool, scale: u64, step: u32) -> Result<Vec<Vec<u64>>, L2Error> {
        // Get the appropriate levels map based on is_bid
        let levels_map = if is_bid {
            &self.bid_level_list
        } else {
            &self.ask_level_list
        };

        // Get levels for the given scale, or empty vector if scale doesn't exist
        let levels = levels_map.get(&scale).cloned().unwrap_or_default();

        // Take step number of levels and convert to array format
        let snapshot: Vec<Vec<u64>> = levels
            .iter()
            .take(step as usize)
            .map(|level| {
                vec![
                    level.price,                    // price in 8 decimals
                    level.pqty,          // base amount in 8 decimals (convert from u128 to u64)
                    level.cqty,          // base amount in 8 decimals (convert from u128 to u64)
                ]
            })
            .collect();

        Ok(snapshot)
    }

    /// get L2 snapshot (formatted strings)
    /// Returns an array of arrays where each inner array is [price as string with 8 decimals, base amount as string with 8 decimals]
    /// The outer array has step length
    /// Numbers are formatted from raw 8-decimal integers to strings with 8 decimal places
    pub fn get_snapshot(&self, is_bid: bool, scale: u64, step: u32) -> Result<Vec<Vec<String>>, L2Error> {
        // Get raw snapshot first
        let raw_snapshot = self.get_snapshot_raw(is_bid, scale, step)?;

        // Convert raw numbers to formatted strings
        let snapshot: Vec<Vec<String>> = raw_snapshot
            .iter()
            .map(|level| {
                vec![
                    Self::format_8_decimals(level[0]),  // format price
                    Self::format_8_decimals(level[1]),  // format public quantity
                    Self::format_8_decimals(level[2]),  // format current quantity
                ]
            })
            .collect();

        Ok(snapshot)
    }
}