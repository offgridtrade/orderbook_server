use serde::{Deserialize, Serialize};

use crate::orderbook::{OrderBook, OrderBookError};
use crate::event::{self, Event};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MatchingEngine {
    pub orderbook: OrderBook,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self { orderbook: OrderBook::new() }
    }

    /// Place a limit sell order (ask order)
    pub fn limit_sell(
        &mut self,
        cid: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let (order_id, found_dormant) = self.orderbook.place_ask(
            cid,
            owner,
            price,
            amount,
            public_amount,
            timestamp,
            expires_at,
        )?;

        // Emit OrderPlaced event
        event::emit_event(Event::OrderPlaced {
            id: order_id as u64,
            qty: amount,
        });

        Ok((order_id, found_dormant))
    }

    /// Place a limit buy order (bid order)
    pub fn limit_buy(
        &mut self,
        cid: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
        expires_at: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let (order_id, found_dormant) = self.orderbook.place_bid(
            cid,
            owner,
            price,
            amount,
            public_amount,
            timestamp,
            expires_at,
        )?;

        // Emit OrderPlaced event
        event::emit_event(Event::OrderPlaced {
            id: order_id as u64,
            qty: amount,
        });

        Ok((order_id, found_dormant))
    }

    /// Execute a market sell order
    pub fn market_sell(
        &mut self,
        order_id: u32,
        amount: u64,
        clear: bool,
        taker_fee_bps: u16,
    ) -> Result<(u64, u64, u64, u64), OrderBookError> {
        // Execute the order (is_bid = false for sell)
        let (amount_to_send, delete_price, base_fee, quote_fee) = 
            self.orderbook.execute(false, order_id, amount, clear, taker_fee_bps)?;

        // TODO: When matching happens, emit OrderMatched event
        // This would require tracking maker order IDs during matching
        // event::emit_event(Event::OrderMatched {
        //     maker_id: maker_order_id as u64,
        //     taker_id: order_id as u64,
        //     qty: matched_amount,
        // });

        Ok((amount_to_send, delete_price, base_fee, quote_fee))
    }

    /// Execute a market buy order
    pub fn market_buy(
        &mut self,
        order_id: u32,
        amount: u64,
        clear: bool,
        taker_fee_bps: u16,
    ) -> Result<(u64, u64, u64, u64), OrderBookError> {
        // Execute the order (is_bid = true for buy)
        let (amount_to_send, delete_price, base_fee, quote_fee) = 
            self.orderbook.execute(true, order_id, amount, clear, taker_fee_bps)?;

        // TODO: When matching happens, emit OrderMatched event
        // This would require tracking maker order IDs during matching
        // event::emit_event(Event::OrderMatched {
        //     maker_id: maker_order_id as u64,
        //     taker_id: order_id as u64,
        //     qty: matched_amount,
        // });

        Ok((amount_to_send, delete_price, base_fee, quote_fee))
    }

    /// Match orders at a specific price level
    fn _match_at() {
        // TODO: Implement matching logic
        // When orders are matched, emit OrderMatched event
    }

    /// Place a limit order (internal helper)
    fn _limit_order() {
        // TODO: Implement limit order logic
    }
}