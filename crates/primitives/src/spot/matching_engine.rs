use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::spot::event::SpotEvent;

use super::event::{self, EventQueue};
use super::orderbook::{OrderBookError, OrderMatch};
use super::orders::OrderId;
use super::pair::Pair;
use super::time_in_force::TimeInForce;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchingEngine {
    pairs: HashMap<Vec<u8>, Pair>,
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

    pub fn add_pair(&mut self, cid: impl Into<Vec<u8>>, client_admin_account_id: impl Into<Vec<u8>>, client_fee_account_id: impl Into<Vec<u8>>, pair_id: impl Into<Vec<u8>>, timestamp: i64) {
        // check if the pair already exists
        let pair_id_vec = pair_id.into();
        if self.pairs.contains_key(&pair_id_vec) {
            // add the client to the pair
            let cid_vec = cid.into();
            self.pairs.get_mut(&pair_id_vec).unwrap().add_client(cid_vec.clone(), client_admin_account_id, client_fee_account_id);
            // emit the event
            event::emit_event(SpotEvent::SpotPairAdded {
                cid: cid_vec,
                pair_id: pair_id_vec,
                timestamp: timestamp,
            });
            return;
        }

        // create the pair
        let mut pair = Pair::new();
        pair.pair_id = pair_id_vec.clone();
        let cid_vec = cid.into();
        pair.add_client(cid_vec.clone(), client_admin_account_id, client_fee_account_id);
        // emit the event
        event::emit_event(SpotEvent::SpotPairAdded {
            cid: cid_vec,
            pair_id: pair_id_vec,
            timestamp: timestamp,
        });
    }

    pub fn add_pair_client(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        admin_account_id: impl Into<Vec<u8>>,
        fee_account_id: impl Into<Vec<u8>>,
    ) -> Result<EventQueue, OrderBookError> {
        let pair_id_vec = pair_id.into();
        let pair = self.pairs.get_mut(&pair_id_vec).unwrap();
        pair.add_client(cid.into(), admin_account_id, fee_account_id);
        Ok(event::drain_events())
    }

    /// Place a limit sell order (ask order)
    /// Matches against existing orders first, then places remaining in orderbook based on time_in_force
    ///
    /// Returns `((order_id, found_dormant), events)` where:
    /// - `order_id`: The order ID of the placed order
    /// - `found_dormant`: Whether a dormant order was found and reused
    /// - `events`: Vector of events emitted during this operation
    pub fn limit_sell(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        existing_order_id: Option<OrderId>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        // whole amount
        amnt: u64,
        // iceberg quantity
        iqty: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<EventQueue, OrderBookError> {
        // find a pair
        let pair_id_vec = pair_id.into();
        let pair = self.pairs.get_mut(&pair_id_vec).unwrap();
        pair.limit_sell(cid, existing_order_id, owner, price, amnt, iqty, timestamp, expires_at, maker_fee_bps, taker_fee_bps, time_in_force)?;
        
        // Drain all events that were emitted during this operation
        let events = event::drain_events();
        
        Ok(events)
    }

    /// Place a limit buy order (bid order)
    /// Matches against existing orders first, then places remaining in orderbook based on time_in_force
    ///
    /// Returns `((order_id, found_dormant), events)` where:
    /// - `order_id`: The order ID of the placed order
    /// - `found_dormant`: Whether a dormant order was found and reused
    /// - `events`: Vector of events emitted during this operation
    pub fn limit_buy(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        existing_order_id: Option<OrderId>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<EventQueue, OrderBookError> {
        // find a pair
        let pair_id_vec = pair_id.into();
        let pair = self.pairs.get_mut(&pair_id_vec).unwrap();
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
        )?;
        
        // Drain all events that were emitted during this operation
        let events = event::drain_events();
        
        Ok(events)
    }

    /// Execute a market sell order
    /// Matches against existing orders first (market orders match at any price)
    ///
    /// Returns `(OrderMatch, events)` where:
    /// - `OrderMatch`: Contains trade execution details
    /// - `events`: Vector of events emitted during this operation
    pub fn market_sell(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        existing_order_id: Option<OrderId>,
        owner: impl Into<Vec<u8>>,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<EventQueue, OrderBookError> {
        let pair_id_vec = pair_id.into();
        let pair = self.pairs.get_mut(&pair_id_vec).unwrap();
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
        )?;
        
        // Drain all events that were emitted during this operation
        let events = event::drain_events();
        
        Ok(events)
    }

    /// Execute a market buy order
    /// Matches against existing orders first (market orders match at any price)
    ///
    /// Returns `(OrderMatch, events)` where:
    /// - `OrderMatch`: Contains trade execution details
    /// - `events`: Vector of events emitted during this operation
    pub fn market_buy(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        existing_order_id: Option<OrderId>,
        owner: impl Into<Vec<u8>>,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<EventQueue, OrderBookError> {
        let pair_id_vec = pair_id.into();
        let pair = self.pairs.get_mut(&pair_id_vec).unwrap();
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
        )?;
        
        // Drain all events that were emitted during this operation
        let events = event::drain_events();
        
        Ok(events)
    }

    /// Cancel an order
    ///
    /// Returns `events` - Vector of events emitted during this operation
    ///
    /// - `order_id`: The ID of the order to cancel
    /// - `owner`: The owner of the order (for authorization)
    /// - `is_bid`: Whether the order is a bid (true) or ask (false)
    pub fn cancel_order(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        order_id: OrderId,
        owner: impl Into<Vec<u8>>,
        is_bid: bool,
        ) -> Result<EventQueue, OrderBookError> {
        let pair_id_vec = pair_id.into();
        let pair = self.pairs.get_mut(&pair_id_vec).unwrap();
        pair.cancel_order(cid, pair_id_vec, is_bid, order_id, owner)?;
        
        // Drain all events that were emitted during this operation
        let events = event::drain_events();
        
        Ok(events)
    }

    /// Get the number of pairs in the matching engine
    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }

    /// Check if a pair exists
    pub fn has_pair(&self, pair_id: &Vec<u8>) -> bool {
        self.pairs.contains_key(&pair_id.clone())
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}
