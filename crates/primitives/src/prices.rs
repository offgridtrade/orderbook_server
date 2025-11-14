use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Level {
    pub price: u64,
    pub quantity: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct L2 {
    /// Head of the bid price linked list
    pub bid_price_head: Option<u64>,
    /// Head of the ask price linked list
    pub ask_price_head: Option<u64>,
    /// Mapping price -> head of the bid price linked list
    pub bid_price_lists: BTreeMap<u64, u64>,
    /// Mapping price -> head of the ask price linked list
    pub ask_price_lists: BTreeMap<u64, u64>,
    /// Bid levels sorted by price descending
    pub bid_level: BTreeMap<u64, u64>,
    /// Ask levels sorted by price ascending
    pub ask_level: BTreeMap<u64, u64>,
    /// Bid levels sorted by price descending for snapshot display
    /// key is scale in 8 decimals integer (e.g. 100000000 for 1.00000000, 1000000000 for 10.00000000)
    /// value is a vector of levels in the quantized price space
    pub bid_levels: BTreeMap<u64, Vec<Level>>,
    /// Ask levels sorted by price ascending for snapshot display
    /// key is scale in 8 decimals integer (e.g. 100000000 for 1.00000000, 1000000000 for 10.00000000)
    /// value is a vector of levels in the quantized price space
    pub ask_levels: BTreeMap<u64, Vec<Level>>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum L2Error {
    #[error("price is zero")]
    PriceIsZero,
}

impl L2 {
    pub fn new() -> Self {
        Self {
            bid_price_head: None,
            ask_price_head: None,
            bid_price_lists: BTreeMap::new(),
            ask_price_lists: BTreeMap::new(),
            bid_level: BTreeMap::new(),
            ask_level: BTreeMap::new(),
            bid_levels: BTreeMap::new(),
            ask_levels: BTreeMap::new(),
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

    pub fn bid_level(&self, price: u64) -> Option<u64> {
        self.bid_level.get(&price).copied()
    }

    pub fn ask_level(&self, price: u64) -> Option<u64> {
        self.ask_level.get(&price).copied()
    }

    pub fn bid_levels(&self, scale: u64) -> &Vec<Level> {
        &self.bid_levels.get(&scale).unwrap()
    }

    pub fn ask_levels(&self, scale: u64) -> &Vec<Level> {
        &self.ask_levels.get(&scale).unwrap()
    }

    pub fn scale_bid_levels(&self, scale: u64, n: u32) -> Vec<Level> {
        let levels = self.bid_levels.get(&scale).cloned().unwrap_or(Vec::new());
        levels.iter().take(n as usize).cloned().collect()
    }

    pub fn scale_ask_levels(&self, scale: u64, n: u32) -> Vec<Level> {
        let levels = self.ask_levels.get(&scale).cloned().unwrap_or(Vec::new());
        levels.iter().take(n as usize).cloned().collect()
    }

    pub fn set_bid_level(&mut self, price: u64, level: u64) {
        self.bid_level.insert(price, level);
    }

    pub fn set_ask_level(&mut self, price: u64, level: u64) {
        self.ask_level.insert(price, level);
    }

    pub fn set_bid_levels(&mut self, scale: u64, levels: Vec<Level>) {
        self.bid_levels.insert(scale, levels);
    }

    pub fn set_ask_levels(&mut self, scale: u64, levels: Vec<Level>) {
        self.ask_levels.insert(scale, levels);
    }

    pub fn insert_price(&mut self, is_bid: bool, price: u64) -> Result<(), L2Error> {
        if is_bid {
            self._insert_bid_price(price)
        }
        else {
            self._insert_ask_price(price)
        }
    }

    /// inserts a bid price into the bid price linked list
    /// price linked list is sorted in descending order
    fn _insert_bid_price(&mut self, price: u64) -> Result<(), L2Error> {
        Self::ensure_price(price)?;
        // compare head of the bid price head
        if self.bid_price_head == None {
            self.bid_price_head = Some(price);
            return Ok(());
        }
        else if price > self.bid_price_head.unwrap() {
            let old_head = self.bid_price_head.unwrap();
            self.bid_price_head = Some(price);
            self.bid_price_lists.insert(price, old_head);
            return Ok(());
        } 
        else if price == self.bid_price_head.unwrap() {
            return Ok(());
        }
        else {
            // traverse through the bid price linked list and insert the price at the correct position so that the list is sorted in descending order
            let mut current = self.bid_price_head;
            while current.is_some() {
                let next = self.bid_price_lists.get(&current.unwrap());
                // next does not exist
                if next.is_none() {
                    // insert the price at the current position
                    self.bid_price_lists.insert(current.unwrap(), price);
                    return Ok(());
                }
                else {
                    // next exists
                    if *next.unwrap() > price {
                        // traverse until the price is bigger than the current price
                        current = Some(*next.unwrap());
                    }
                    else if *next.unwrap() < price {
                        // To avoid mutable and immutable borrow at the same time, collect value first
                        let curr = current.unwrap();
                        let next_val = *next.unwrap();
                        // remove the link between the current price and the next price
                        self.bid_price_lists.remove(&curr);
                        // insert the link between the current price and the new price
                        self.bid_price_lists.insert(curr, price);
                        // insert the link between the current price and the next price
                        self.bid_price_lists.insert(price, next_val);
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
            return Ok(());
        }
        else if price < self.ask_price_head.unwrap() {
            let old_head = self.ask_price_head.unwrap();
            self.ask_price_head = Some(price);
            self.ask_price_lists.insert(price, old_head);
            return Ok(());
        }
        else if price == self.ask_price_head.unwrap() {
            return Ok(());
        }
        else {
            // traverse through the ask price linked list and insert the price at the correct position so that the list is sorted in ascending order
            let mut current = self.ask_price_head;
            while current.is_some() {
                let next = self.ask_price_lists.get(&current.unwrap());
                // next does not exist
                if next.is_none() {
                    // insert the price at the current position
                    self.ask_price_lists.insert(current.unwrap(), price);
                    return Ok(());
                }
                else {
                    // next exists
                    if *next.unwrap() < price {
                        // traverse until the price is smaller than the current price
                        current = Some(*next.unwrap());
                    }
                    else if *next.unwrap() > price {
                        // To avoid mutable and immutable borrow at the same time, collect value first
                        let curr = current.unwrap();
                        let next_val = *next.unwrap();
                        // remove the link between the current price and the next price
                        self.ask_price_lists.remove(&curr);
                        // insert the link between the current price and the new price
                        self.ask_price_lists.insert(curr, price);
                        // insert the link between the current price and the next price
                        self.ask_price_lists.insert(price, next_val);
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
}