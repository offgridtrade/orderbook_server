use serde::{Deserialize, Serialize};

use crate::orderbook::OrderBook;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MatchingEngine {
    pub orderbook: OrderBook,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self { orderbook: OrderBook::new() }
    }

    pub fn limit_sell() {}

    pub fn limit_buy() {}

    pub fn market_sell() {}

    pub fn market_buy() {}

    fn _match_at() {}

    fn _limit_order() {}
}