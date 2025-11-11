use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub price: u64,
    pub quantity: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct L2 {
    /// Mapping price -> head of the bid list
    pub bid_price_lists: BTreeMap<u64, u64>,
    /// Mapping price -> head of the ask list
    pub ask_price_lists: BTreeMap<u64, u64>,
    /// Bid levels sorted by price descending for snapshot display 
    pub bids: Vec<Level>,
    /// Ask levels sorted by price ascending for snapshot display
    pub asks: Vec<Level>,
}
