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

    /// Gets the required amount of base liquidity to match an order and clear the order.
    pub fn get_required(&self, order_id: u32) -> Result<u64, OrderBookError> {
        let order = self.l3.get_order(order_id)?;
        Ok(order.cq)
    }


    /// clears empty head of the order book where price is in linked list, but order is not in the price level
    pub fn clear_empty_head(&mut self, is_bid: bool) -> Result<u64, OrderBookError> {
        // Get the current head price
        let mut head = if is_bid { self.l2.bid_head() } else { self.l2.ask_head() };
        
        // While head exists and has no orders, clear it and move to the next head
        while let Some(head_price) = head {
            // Check if there are orders at this price level
            let order_id = self.l3.head(head_price);
            
            // If there are orders at this price level, we're done
            if order_id.is_some() {
                return Ok(head_price);
            }
            
            // No orders at this price level, clear the head and move to next
            head = self.l2.clear_head(is_bid)?;
        }
        
        // No head exists (all heads were empty and cleared)
        Err(OrderBookError::PriceIsZero)
    }

    /// pop front on the orderbook 
    pub fn pop_front(&mut self, is_bid: bool) -> Result<u32, OrderBookError> {
        self.clear_empty_head(is_bid)?;
        let head = if is_bid { self.l2.bid_head() } else { self.l2.ask_head() };
        
        let head_price = head.unwrap();
        let (order_id, is_empty) = self.l3.pop_front(head_price)?;
        if is_empty {
            self.l2.clear_head(is_bid)?;
        }
        Ok(order_id.unwrap())
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
        expires_at: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let owner = owner.into();
        let price = price;
        let amount = amount;
        let (id, found_dormant) = self.l3.create_order(cid, owner, price, amount, public_amount, timestamp, expires_at)?;
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
        expires_at: i64,
    ) -> Result<(u32, bool), OrderBookError> {
        let owner = owner.into();
        let price = price.into();
        let amount = amount.into();
        let (id, found_dormant) = self.l3.create_order(cid, owner, price, amount, public_amount, timestamp, expires_at)?;
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
    /// when the order is cleared, the price level is updated on bid and ask side.
    /// the function returns base and quote amount to send to the taker and maker.
    /// the function also returns the base fee and quote fee to send to the protocol.
    pub fn execute(
        &mut self,
        is_bid: bool,
        order_id: u32,
        amount: u64,
        clear: bool,
        taker_fee_bps: u16,
    ) -> Result<(u64, u64, u64, u64), OrderBookError> {
        let amount = amount;
        let clear = clear;
        let (amount_to_send, delete_price) = self.l3.decrease_order(order_id, amount, 1u64, clear)?;
        // adjust price level on the matched amount
        Ok((amount_to_send, delete_price.unwrap(), base_fee, quote_fee))
    }

    fn _get_sent_funds() {

    }

    fn _get_fee() {

    }

    fn _update_levels() {

    }

    /// Cancels an order.
    /// - returns the amount to send and the delete price.
    /// - `order_id` is the id of the order to cancel.
    /// - `owner` is the owner of the order.
    pub fn cancel_order(
        &mut self,
        is_bid: bool,
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
        let delete_price = self.l3.delete_order(order_id)?;
        // if delete price is some, delete the price level
        if delete_price.is_some() {
            self.l2.remove_price(is_bid, delete_price.unwrap())?;
        }
        Ok(())
    }
}