use serde::{Deserialize, Serialize};

use crate::{L1, L2, L3, orders::L3Error, prices::L2Error, market::L1Error};

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
    #[error("order is not owned by the sender")]
    OrderNotOwnedBySender,
    #[error("L3 error: {0}")]
    L3(L3Error),
    #[error("L2 error: {0}")]
    L2(L2Error),
    #[error("L1 error: {0}")]
    L1(L1Error),
}

impl From<L3Error> for OrderBookError {
    fn from(err: L3Error) -> Self {
        OrderBookError::L3(err)
    }
}

impl From<L2Error> for OrderBookError {
    fn from(err: L2Error) -> Self {
        OrderBookError::L2(err)
    }
}

impl From<L1Error> for OrderBookError {
    fn from(err: L1Error) -> Self {
        OrderBookError::L1(err)
    }
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            l1: L1::new(),
            l2: L2::new(),
            l3: L3::new(),
        }
    }

    /// Gets the required amount of liquidity to match an order and clear the order.
    pub fn get_required() {
        
    }

    pub fn clear_empty_head(&mut self, is_bid: bool) -> Result<u64, OrderBookError> {
        let head = if is_bid { self.l2.bid_head() } else { self.l2.ask_head() };
        let order_id = if is_bid { self.l3.head(head.unwrap()) } else { self.l3.head(head.unwrap()) };
        while order_id.is_some() && head.is_some() {
            let head = if is_bid { self.l2.bid_head() } else { self.l2.ask_head() };
            let order_id = if is_bid { self.l3.head(head.unwrap()) } else { self.l3.head(head.unwrap()) };
        }
        let delete_price = self.l2.clear_head(is_bid)?;
        Ok(delete_price.unwrap())
    }

    pub fn fpop() {

    }

    /// Places a bid order.
    /// - returns the order id and if a dormant order was found.
    /// - `cid` is the client order id.
    /// - `owner` is the owner of the order.
    /// - `price` is the price of the order.
    /// - `amount` is the amount of the order.
    /// - `public_amount` is the public amount of the order.
    /// - `timestamp` is the timestamp of the order.
    pub fn place_bid(
        &mut self,
        cid: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let owner = owner.into();
        let price = price;
        let amount = amount;
        let (id, found_dormant) = self.l3.create_order(cid, owner, price, amount, public_amount, timestamp)?;
        Ok((id, found_dormant))
    }

    /// Places an ask order.
    /// - returns the order id and if a dormant order was found.
    /// - `cid` is the client order id.
    /// - `owner` is the owner of the order.
    /// - `price` is the price of the order.
    /// - `amount` is the amount of the order.
    /// - `public_amount` is the public amount of the order.
    /// - `timestamp` is the timestamp of the order.
    pub fn place_ask(
        &mut self,
        cid: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amount: u64,
        public_amount: u64,
        timestamp: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let owner = owner.into();
        let price = price.into();
        let amount = amount.into();
        let (id, found_dormant) = self.l3.create_order(cid, owner, price, amount, public_amount, timestamp)?;
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
    pub fn execute(
        &mut self,
        order_id: u32,
        amount: u64,
        clear: bool
    ) -> Result<(u64, u64), OrderBookError> {
        let amount = amount;
        let clear = clear;
        let (amount_to_send, delete_price) = self.l3.decrease_order(order_id, amount, 1u64, clear)?;
        Ok((amount_to_send, delete_price.unwrap()))
    }

    /// Cancels an order.
    /// - returns the amount to send and the delete price.
    /// - `order_id` is the id of the order to cancel.
    /// - `owner` is the owner of the order.
    pub fn cancel_order(
        &mut self,
        order_id: u32,
        owner: impl Into<Vec<u8>>,
    ) -> Result<(), OrderBookError> {
        let owner = owner.into();
        // check if the order exists
        let order = self.l3.get_order(order_id)?;
        // check if the owner is the same as the owner of the order
        if order.owner != owner {
            return Err(OrderBookError::OrderNotOwnedBySender);
        }
        let _delete_price = self.l3.delete_order(order_id)?.ok_or(L3Error::OrderDoesNotExist(order_id))?;
        Ok(())
    }
}