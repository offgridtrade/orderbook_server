use serde::{Deserialize, Serialize};

use crate::{L1, L2, L3};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrderBook {
    pub l1: L1,
    pub l2: L2,
    pub l3: L3,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrderBookError {
    #[error("price is zero")]
    PriceIsZero,
    #[error("amount is zero")]
    AmountIsZero,
    #[error("public amount is zero")]
    PublicAmountIsZero,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            l1: L1::new(),
            l2: L2::new(),
            l3: L3::new(),
        }
    }

    pub fn getRequired() {
        
    }

    pub fn fpop() {

    }

    /// Places a bid order.
    /// - returns the order id and if a dormant order was found.
    /// - `owner` is the owner of the order.
    /// - `price` is the price of the order.
    /// - `amount` is the amount of the order.
    /// - `public_amount` is the public amount of the order.
    /// - `timestamp` is the timestamp of the order.
    pub fn place_bid(
        &self,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let owner = owner.into();
        let price = price.into();
        let amount = amount.into();
        let (id, found_dormant) = self.l3.create_order(owner, price, amount)?;
        Ok((id, found_dormant))
    }

    /// Places an ask order.
    /// - returns the order id and if a dormant order was found.
    /// - `owner` is the owner of the order.
    /// - `price` is the price of the order.
    /// - `amount` is the amount of the order.
    /// - `public_amount` is the public amount of the order.
    /// - `timestamp` is the timestamp of the order.
    pub fn place_ask(
        &self,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let owner = owner.into();
        let price = price.into();
        let amount = amount.into();
        let (id, found_dormant) = self.l3.create_order(owner, price, amount)?;
        Ok((id, found_dormant))
    }

    /// Executes a trade.
    /// - returns the trade details.
    /// - `bid_order_id` is the id of the bid order.
    /// - `ask_order_id` is the id of the ask order.
    /// - `price` is the price of the trade.
    /// - `amount` is the amount of the trade.
    /// - `public_amount` is the public amount of the trade.
    /// - `timestamp` is the timestamp of the trade.
    pub fn execute() {

    }
}