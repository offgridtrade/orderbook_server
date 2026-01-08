use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::orderbook::{OrderBookError, OrderMatch};
use crate::pair::Pair;
use crate::time_in_force::TimeInForce;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchingEngine {
    pairs: HashMap<String, Pair>,
    total_pairs: u32,
}

impl MatchingEngine {
    /// Create a new exchange instance
    pub fn new() -> Self {
        Self {
            pairs: HashMap::new(),
            total_pairs: 0,
        }
    }

    pub fn add_pair(&mut self, pair_id: impl Into<String>) {
        self.pairs.insert(pair_id.into(), Pair::new());
        self.total_pairs += 1;
    }

    /// Place a limit sell order (ask order)
    /// Matches against existing orders first, then places remaining in orderbook based on time_in_force
    ///
    /// Returns `(order_id, found_dormant)` where:
    /// - `order_id`: The order ID of the placed order
    /// - `found_dormant`: Whether a dormant order was found and reused
    pub fn limit_sell(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<String>,
        existing_order_id: Option<u32>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<(u32, bool), OrderBookError> {
        // find a pair
        let pair = self.pairs.get_mut(&pair_id.into()).unwrap();
        pair.limit_sell(
            cid,
            existing_order_id,
            owner,
            price,
            amount,
            public_amount,
            timestamp,
            expires_at,
            maker_fee_bps,
            taker_fee_bps,
            time_in_force,
        )
    }

    /// Place a limit buy order (bid order)
    /// Matches against existing orders first, then places remaining in orderbook based on time_in_force
    ///
    /// Returns `(order_id, found_dormant)` where:
    /// - `order_id`: The order ID of the placed order
    /// - `found_dormant`: Whether a dormant order was found and reused
    pub fn limit_buy(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<String>,
        existing_order_id: Option<u32>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<(u32, bool), OrderBookError> {
        // find a pair
        let pair = self.pairs.get_mut(&pair_id.into()).unwrap();
        pair.limit_buy(
            cid,
            existing_order_id,
            owner,
            price,
            amount,
            public_amount,
            timestamp,
            expires_at,
            maker_fee_bps,
            taker_fee_bps,
            time_in_force,
        )
    }

    /// Execute a market sell order
    /// Matches against existing orders first (market orders match at any price)
    ///
    /// Returns `OrderMatch` containing trade execution details
    pub fn market_sell(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<String>,
        existing_order_id: Option<u32>,
        owner: impl Into<Vec<u8>>,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<OrderMatch, OrderBookError> {
        let pair = self.pairs.get_mut(&pair_id.into()).unwrap();
        pair.market_sell(
            cid,
            existing_order_id,
            owner,
            amount,
            public_amount,
            timestamp,
            expires_at,
            maker_fee_bps,
            taker_fee_bps,
            time_in_force,
        )
    }

    /// Execute a market buy order
    /// Matches against existing orders first (market orders match at any price)
    ///
    /// Returns `OrderMatch` containing trade execution details
    pub fn market_buy(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<String>,
        existing_order_id: Option<u32>,
        owner: impl Into<Vec<u8>>,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<OrderMatch, OrderBookError> {
        let pair = self.pairs.get_mut(&pair_id.into()).unwrap();
        pair.market_buy(
            cid,
            existing_order_id,
            owner,
            amount,
            public_amount,
            timestamp,
            expires_at,
            maker_fee_bps,
            taker_fee_bps,
            time_in_force,
        )
    }

    /// Cancel an order
    ///
    /// - `order_id`: The ID of the order to cancel
    /// - `owner`: The owner of the order (for authorization)
    /// - `is_bid`: Whether the order is a bid (true) or ask (false)
    pub fn cancel_order(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<String>,
        order_id: u32,
        owner: impl Into<Vec<u8>>,
        is_bid: bool,
    ) -> Result<(), OrderBookError> {
        let pair = self.pairs.get_mut(&pair_id.into()).unwrap();
        pair.orderbook.cancel_order(cid, is_bid, order_id, owner)
    }

    /// Get the number of pairs in the matching engine
    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }

    /// Check if a pair exists
    pub fn has_pair(&self, pair_id: &str) -> bool {
        self.pairs.contains_key(pair_id)
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}
