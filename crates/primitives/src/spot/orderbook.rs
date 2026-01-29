use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::spot::{
    event::{self, SpotEvent},
    Order,
};

use super::{
    orders::{L3Error, OrderId},
    prices::L2Error,
    L2, L3,
};

/// In-memory order book for spot markets.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use offgrid_primitives::spot::orderbook::OrderBook;
///
/// // Create a new, empty orderbook
/// let mut ob = OrderBook::new();
/// assert_eq!(ob.lmp(), None);
///
/// assert_eq!(ob.lmp(), Some(100));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrderBook {
    // L2 state
    pub l2: L2,
    // L3 state
    pub l3: L3,
    // Fee recipients map where key is the client id, and value is the fee recipient account id
    pub fee_recipients: HashMap<Vec<u8>, Vec<u8>>,
    // dust limit to determine if the order should be deleted
    pub dust: u64,
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
    #[error("iceberg quantity is bigger than whole amount")]
    IcebergQuantityIsBiggerThanWholeAmount,
    #[error("order has expired")]
    OrderExpired,
    #[error("unsupported time in force is provided")]
    UnsupportedTimeInForce,
    #[error("order is not supported by the client id")]
    OrderNotSupportedByClientId,
    #[error("fill or kill order not fully filled")]
    OrderNotFullyFilled,
    #[error("no ask orders in the orderbook")]
    NoAskOrdersInOrderbook,
    #[error("no bid orders in the orderbook")]
    NoBidOrdersInOrderbook,
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

impl OrderBook {
    pub fn new() -> Self {
        Self {
            l2: L2::new(),
            l3: L3::new(),
            fee_recipients: HashMap::new(),
            dust: 1000,
        }
    }

    /// Sets the dust limit to determine if the order should be deleted
    pub fn set_dust(&mut self, dust: u64) {
        self.dust = dust;
    }

    /// Gets the required amount to match an order as taker to match with the maker order and clear it.
    /// - `taker_order` is the taker order.
    /// - `price` is the price of the maker order.
    /// - `amount` is the amount of the taker order to match with the maker order.
    pub fn get_required(
        &self,
        taker_order: Order,
        price: u64,
        amount: u64,
    ) -> Result<u64, OrderBookError> {
        if taker_order.is_bid {
            Ok(amount.saturating_mul(price).saturating_div(1_0000_0000))
        } else {
            Ok(amount.saturating_mul(1_0000_0000).saturating_div(price))
        }
    }

    /// clears empty head of the order book where price is in linked list, but order is not in the price level
    pub fn clear_empty_head(&mut self, is_bid: bool) -> Result<u64, OrderBookError> {
        // Get the current head price
        let mut head = if is_bid {
            self.l2.bid_head()
        } else {
            self.l2.ask_head()
        };

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

    /// clears empty head of the order book, returns 0 if no head exists (matches Solidity behavior)
    pub fn clear_empty_head_or_zero(&mut self, is_bid: bool) -> u64 {
        self.clear_empty_head(is_bid).unwrap_or(0)
    }

    /// pop front on the orderbook
    pub fn pop_front(&mut self, is_bid: bool) -> Result<Order, OrderBookError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        loop {
            self.clear_empty_head(is_bid)?;
            let head = if is_bid {
                self.l2.bid_head()
            } else {
                self.l2.ask_head()
            };

            let head_price = match head {
                Some(price) => price,
                None => continue,
            };
            let head_id = self.l3.head(head_price);
            if let Some(order_id) = head_id {
                let order = self.l3.get_order(order_id)?;
                // if the order is expired, expire it and continue
                if order.expires_at <= now {
                    self._expire_order(order_id, is_bid, Vec::new(), now)?;
                    continue;
                }
                // if the expired order empties the price level, remove the price level, move to next head and continue
                if self.l3.is_empty(order.price) {
                    self.l2.remove_price(is_bid, order.price)?;
                    continue;
                }
                // if the order is not expired, pop it from the orderbook
                let (order, is_empty) = self.l3.pop_front(head_price)?;
                if is_empty {
                    self.l2.clear_head(is_bid)?;
                }
                return Ok(order
                    .expect("head price must have at least one order")
                    .clone());
            }
        }
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
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amnt: u64,
        iqty: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
    ) -> Result<Order, OrderBookError> {
        let cid = cid.into();
        let pair_id = pair_id.into();
        let base_asset_id = base_asset_id.into();
        let quote_asset_id = quote_asset_id.into();
        let owner = owner.into();
        let price = price;
        if iqty > amnt {
            return Err(OrderBookError::IcebergQuantityIsBiggerThanWholeAmount);
        }
        let pqty = amnt - iqty;

        let order = self.l3.create_order(
            cid.clone(),
            owner.clone(),
            true,
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            maker_fee_bps,
        )?;

        // emit the event for the order created
        event::emit_event(SpotEvent::SpotOrderPlaced {
            cid: cid.clone(),
            pair_id: pair_id.clone(),
            base_asset_id: base_asset_id.clone(),
            quote_asset_id: quote_asset_id.clone(),
            order_id: order.id.to_bytes().to_vec(),
            maker_account_id: owner.into(),
            is_bid: true,
            price: price,
            amnt: amnt,
            iqty: iqty,
            pqty: pqty,
            cqty: amnt,
            timestamp: timestamp,
            expires_at: expires_at,
        });

        // update the price level on the orderbook
        self.update_price_level(pair_id, true, true, price, pqty, amnt, None)?;
        Ok(order)
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
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        amnt: u64,
        iqty: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
    ) -> Result<Order, OrderBookError> {
        let cid = cid.into();
        let pair_id = pair_id.into();
        let base_asset_id = base_asset_id.into();
        let quote_asset_id = quote_asset_id.into();
        let owner = owner.into();
        let price = price.into();
        let amnt = amnt.into();
        let iqty = iqty.into();
        if iqty > amnt {
            return Err(OrderBookError::IcebergQuantityIsBiggerThanWholeAmount);
        }
        let pqty = amnt - iqty;

        let order = self.l3.create_order(
            cid.clone(),
            owner.clone(),
            false,
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            maker_fee_bps,
        )?;

        // emit the event for the order created
        event::emit_event(SpotEvent::SpotOrderPlaced {
            cid: cid.clone(),
            pair_id: pair_id.clone(),
            base_asset_id: base_asset_id.clone(),
            quote_asset_id: quote_asset_id.clone(),
            order_id: order.id.to_bytes().to_vec(),
            maker_account_id: owner,
            is_bid: false,
            price: price,
            amnt: amnt,
            iqty: iqty,
            pqty: pqty,
            cqty: amnt,
            timestamp: timestamp,
            expires_at: expires_at,
        });

        // update the price level on the orderbook
        self.update_price_level(pair_id, true, false, price, pqty, amnt, None)?;
        Ok(order)
    }

    /// Executes a trade.
    /// - returns the trade details.
    /// - `is_bid` is whether the order from client is a bid order.
    /// - `taker_order` is the taker order.
    /// - `maker_order` is the maker order.
    /// - `pair_id` is the pair id.
    /// - `base_asset_id` is the base asset id.
    /// - `quote_asset_id` is the quote asset id.
    /// - `managing_account_id` is the managing account id.
    /// - `matching_amount` is the amount of the taker order to match with the maker order.
    /// - `clear` is whether to clear the order.
    pub fn execute(
        &mut self,
        taker_order: Order,
        maker_order: Order,
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        now: i64,
    ) -> Result<(), OrderBookError> {
        // Normalize IDs up front so we don't move the Into<Vec<u8>> values multiple times
        let pair_id_vec = pair_id.into();
        let base_asset_id_vec = base_asset_id.into();
        let quote_asset_id_vec = quote_asset_id.into();
        let taker_is_bid = taker_order.is_bid;
        let (matching_amount, taker_clear, maker_clear) = self._get_matching_amount(taker_order.clone(), maker_order.clone())?;
        // matching_amount is expressed in taker terms; convert to base/quote by side
        let matching_base_amount = if taker_is_bid {
            matching_amount.saturating_mul(1_0000_0000).saturating_div(taker_order.price)
        } else {
            matching_amount
        };
        let matching_quote_amount = if taker_is_bid {
            matching_amount
        } else {
            matching_amount.saturating_mul(taker_order.price).saturating_div(1_0000_0000)
        };

        let taker_matching_amount = if taker_is_bid { matching_quote_amount.clone() } else { matching_base_amount.clone() };
        let maker_matching_amount = if taker_is_bid { matching_base_amount.clone() } else { matching_quote_amount.clone() };

        // Get order data before mutable borrow
        if maker_order.expires_at <= now {
            self._expire_order(maker_order.id, !taker_is_bid, pair_id_vec.clone(), now)?;
            // let _match_at at pair.rs handle the expired order error
            return Err(OrderBookError::OrderExpired);
        }
        let (_taker_delete_amount, taker_delete_price) =
            self.l3
                .decrease_order(taker_order.id, taker_matching_amount, self.dust, taker_clear)?;
        let (_maker_delete_amount, maker_delete_price) =
            self.l3
                .decrease_order(maker_order.id, maker_matching_amount, self.dust, maker_clear)?;

        // Calculate fees using fee table
        let (base_fee, quote_fee) = self._calculate_fees(
            taker_is_bid,
            matching_base_amount,
            matching_quote_amount,
            maker_order.fee_bps,
            taker_order.fee_bps,
        );

        // emit the event for order matched
        let match_timestamp = now;

        let (taker_remaining_cqty, taker_remaining_pqty) = match self.l3.get_order(taker_order.id) {
            Ok(updated) => (updated.cqty, updated.pqty),
            Err(_) => (0, 0),
        };

        // emit event for both maker and taker
        let (maker_remaining_cqty, maker_remaining_pqty) = match self.l3.get_order(maker_order.id) {
            Ok(updated) => (updated.cqty, updated.pqty),
            Err(_) => (0, 0),
        };
        let taker_delta_cqty = taker_order.cqty.saturating_sub(taker_remaining_cqty);
        let taker_delta_pqty = taker_order.pqty.saturating_sub(taker_remaining_pqty);
        let maker_delta_cqty = maker_order.cqty.saturating_sub(maker_remaining_cqty);
        let maker_delta_pqty = maker_order.pqty.saturating_sub(maker_remaining_pqty);
        // emit taker / maker order history event
        self._emit_taker_maker_match(
            taker_order.clone(),
            maker_order.clone(),
            taker_remaining_cqty,
            taker_remaining_pqty,
            maker_remaining_cqty,
            maker_remaining_pqty,
            pair_id_vec.clone(),
            base_asset_id_vec.clone(),
            quote_asset_id_vec.clone(),
            matching_base_amount,
            matching_quote_amount,
            base_fee,
            quote_fee,
            match_timestamp,
            taker_order.expires_at,
            maker_order.expires_at,
        )?;

        // adjust price level on the matched amount
        // Update levels and remove price if level becomes 0 or below
        // Also handle delete_price removal if an order was fully consumed
        self.update_price_level(
            pair_id_vec.clone(),
            false,
            taker_order.is_bid,
            taker_order.price,
            taker_delta_pqty,
            taker_delta_cqty,
            taker_delete_price,
        )?;
        self.update_price_level(
            pair_id_vec,
            false,
            maker_order.is_bid,
            maker_order.price,
            maker_delta_pqty,
            maker_delta_cqty,
            maker_delete_price,
        )?;

        Ok(())
    }

    /// Determines the matching amount between the taker and maker orders.
    /// - `taker_order` is the taker order.
    /// - `maker_order` is the maker order.
    /// - returns the matching amount and whether the taker order is fully matched and whether the maker order is fully matched.
    fn _get_matching_amount(
        &mut self,
        taker_order: Order,
        maker_order: Order,
    ) -> Result<(u64, bool, bool), OrderBookError> {
        let taker_converted_matching_cqty = if taker_order.is_bid {
            taker_order.cqty.saturating_mul(1_0000_0000).saturating_div(taker_order.price)
        } else {
            taker_order.cqty.saturating_mul(taker_order.price).saturating_div(1_0000_0000)
        };
        // there are two cases:
        // 1. taker order's converted matching amount is bigger than maker order's matching amount
        if taker_converted_matching_cqty >= maker_order.cqty {
            // get the taker's matching amount from the maker order
            let taker_matching_amount = self.get_required(maker_order.clone(), taker_order.price, maker_order.cqty)?;
            return Ok((taker_matching_amount, false, true));
        } 
        // 2. taker order's converted matching amount is smaller than maker order's matching amount
        else if taker_converted_matching_cqty < maker_order.cqty {
            // get the maker's matching amount from the taker order
            let maker_matching_cqty = self.get_required(taker_order.clone(), maker_order.price, taker_order.cqty)?;
            return Ok((maker_matching_cqty, true, false));
        }
        // 3. taker order's converted matching amount is equal to maker order's matching amount
        else { 
            // get the taker's matching amount from the maker order
            return Ok((taker_order.cqty, true, true)); 
        }
    }

    fn _calculate_fees(
        &self,
        is_bid: bool,
        matching_base_amount: u64,
        matching_quote_amount: u64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
    ) -> (u64, u64) {
        // find maker and taker from base and quote amount
        if is_bid {
            (
                matching_base_amount * maker_fee_bps as u64 / 10000,
                matching_quote_amount * taker_fee_bps as u64 / 10000,
            )
        } else {
            (
                matching_base_amount * taker_fee_bps as u64 / 10000,
                matching_quote_amount * maker_fee_bps as u64 / 10000,
            )
        }
    }

    fn _expire_order(
        &mut self,
        order_id: OrderId,
        is_bid: bool,
        pair_id: Vec<u8>,
        now: i64,
    ) -> Result<(), OrderBookError> {
        let order = self.l3.get_order(order_id)?.clone();
        let deleted_price_opt = self.l3.delete_order(order_id)?;
        // update the price level on the orderbook
        self.update_price_level(
            pair_id,
            false,
            is_bid,
            order.price,
            order.pqty,
            order.cqty,
            deleted_price_opt,
        )?;
        // emit event for the order expired
        event::emit_event(SpotEvent::SpotOrderExpired {
            cid: order.cid.clone(),
            order_id: order_id.to_bytes().to_vec(),
            maker_account_id: order.owner.clone(),
            is_bid,
            price: order.price,
            amnt: order.amnt,
            iqty: order.iqty,
            pqty: order.pqty,
            cqty: order.cqty,
            timestamp: now,
            expires_at: order.expires_at,
        });
        Ok(())
    }

    fn _emit_taker_maker_match(
        &self,
        taker_order: Order,
        maker_order: Order,
        taker_remaining_cqty: u64,
        taker_remaining_pqty: u64,
        maker_remaining_cqty: u64,
        maker_remaining_pqty: u64,
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        matching_base_amount: u64,
        matching_quote_amount: u64,
        base_fee: u64,
        quote_fee: u64,
        timestamp: i64,
        taker_expires_at: i64,
        maker_expires_at: i64,
    ) -> Result<(), OrderBookError> {
        let pair_id_vec = pair_id.into();
        let base_asset_id_vec = base_asset_id.into();
        let quote_asset_id_vec = quote_asset_id.into();

        // emit event for taker order filled
        if taker_remaining_cqty > 0 {
            event::emit_event(SpotEvent::SpotOrderPartiallyFilled {
                is_taker_event: true,
                taker_cid: taker_order.cid.clone(),
                maker_cid: maker_order.cid.clone(),
                taker_order_id: taker_order.id.to_bytes().to_vec(),
                maker_order_id: maker_order.id.to_bytes().to_vec(),
                taker_account_id: taker_order.owner.clone(),
                maker_account_id: maker_order.owner.clone(),
                taker_order_is_bid: taker_order.is_bid,
                maker_order_is_bid: maker_order.is_bid,
                price: taker_order.price,
                pair_id: pair_id_vec.clone(),
                base_asset_id: base_asset_id_vec.clone(),
                quote_asset_id: quote_asset_id_vec.clone(),
                base_volume: matching_base_amount,
                quote_volume: matching_quote_amount,
                base_fee: base_fee,
                quote_fee: quote_fee,
                maker_fee_bps: maker_order.fee_bps,
                taker_fee_bps: taker_order.fee_bps,
                amnt: taker_order.amnt,
                iqty: taker_order.iqty,
                pqty: taker_remaining_pqty,
                cqty: taker_remaining_cqty,
                timestamp: timestamp,
                expires_at: taker_expires_at,
            });
        } else {
            event::emit_event(SpotEvent::SpotOrderFullyFilled {
                is_taker_event: true,
                taker_cid: taker_order.cid.clone(),
                maker_cid: maker_order.cid.clone(),
                taker_order_id: taker_order.id.to_bytes().to_vec(),
                maker_order_id: maker_order.id.to_bytes().to_vec(),
                taker_account_id: taker_order.owner.clone(),
                maker_account_id: maker_order.owner.clone(),
                taker_order_is_bid: taker_order.is_bid,
                maker_order_is_bid: maker_order.is_bid,
                price: taker_order.price,
                pair_id: pair_id_vec.clone(),
                base_asset_id: base_asset_id_vec.clone(),
                quote_asset_id: quote_asset_id_vec.clone(),
                base_volume: matching_base_amount,
                quote_volume: matching_quote_amount,
                base_fee: base_fee,
                quote_fee: quote_fee,
                maker_fee_bps: maker_order.fee_bps,
                taker_fee_bps: taker_order.fee_bps,
                amnt: taker_order.amnt,
                iqty: taker_order.iqty,
                pqty: taker_remaining_pqty,
                cqty: taker_remaining_cqty,
                timestamp: timestamp,
                expires_at: taker_expires_at,
            });
        }

        // emit event for maker order filled
        if maker_remaining_cqty > 0 {
            event::emit_event(SpotEvent::SpotOrderPartiallyFilled {
                is_taker_event: false,
                taker_cid: taker_order.cid.clone(),
                maker_cid: maker_order.cid.clone(),
                taker_order_id: taker_order.id.to_bytes().to_vec(),
                maker_order_id: maker_order.id.to_bytes().to_vec(),
                taker_account_id: taker_order.owner.clone(),
                maker_account_id: maker_order.owner.clone(),
                taker_order_is_bid: taker_order.is_bid,
                maker_order_is_bid: maker_order.is_bid,
                pair_id: pair_id_vec.clone(),
                base_asset_id: base_asset_id_vec.clone(),
                quote_asset_id: quote_asset_id_vec.clone(),
                price: maker_order.price,
                base_volume: matching_base_amount,
                quote_volume: matching_quote_amount,
                base_fee: base_fee,
                quote_fee: quote_fee,
                maker_fee_bps: maker_order.fee_bps,
                taker_fee_bps: taker_order.fee_bps,
                amnt: maker_order.amnt,
                iqty: maker_order.iqty,
                pqty: maker_remaining_pqty,
                cqty: maker_remaining_cqty,
                timestamp: timestamp,
                expires_at: maker_expires_at,
            });
        } else {
            event::emit_event(SpotEvent::SpotOrderFullyFilled {
                is_taker_event: false,
                taker_cid: taker_order.cid.clone(),
                maker_cid: maker_order.cid.clone(),
                taker_order_id: taker_order.id.to_bytes().to_vec(),
                maker_order_id: maker_order.id.to_bytes().to_vec(),
                taker_account_id: taker_order.owner.clone(),
                maker_account_id: maker_order.owner.clone(),
                taker_order_is_bid: taker_order.is_bid,
                maker_order_is_bid: maker_order.is_bid,
                pair_id: pair_id_vec.clone(),
                base_asset_id: base_asset_id_vec.clone(),
                quote_asset_id: quote_asset_id_vec.clone(),
                price: maker_order.price,
                base_volume: matching_base_amount,
                quote_volume: matching_quote_amount,
                base_fee: base_fee,
                quote_fee: quote_fee,
                maker_fee_bps: maker_order.fee_bps,
                taker_fee_bps: taker_order.fee_bps,
                amnt: maker_order.amnt,
                iqty: maker_order.iqty,
                pqty: maker_remaining_pqty,
                cqty: maker_remaining_cqty,
                timestamp: timestamp,
                expires_at: maker_expires_at,
            });
        }
        Ok(())
    }

    /// updates the levels on the orderbook in the price level linked list
    /// - `cid` is the client id.
    /// - `pair_id` is the pair id.
    /// - `is_placed` is whether the order is placed.
    /// - `is_bid` is whether the order is a bid order.
    /// - `price` is the price of the order.
    /// - `delta_pqty` is the delta quantity of the public quantity.
    /// - `delta_cqty` is the delta quantity of the current quantity.
    /// - `delete_price` is an optional price that should be removed (when an order was fully consumed).
    /// Removes the price if the level becomes 0 or below.
    pub fn update_price_level(
        &mut self,
        pair_id: Vec<u8>,
        is_placed: bool,
        is_bid: bool,
        price: u64,
        delta_pqty: u64,
        delta_cqty: u64,
        delete_price: Option<u64>,
    ) -> Result<(), OrderBookError> {
        if is_placed {
            // insert price if the price does not exist
            if !self.l2.price_exists(is_bid, price) {
                self.l2.insert_price(is_bid, price)?;
                // Initialize level to 0 for new price
                if is_bid {
                    self.l2.set_public_bid_level(price, delta_pqty)?;
                    self.l2.set_current_bid_level(price, delta_cqty)?;
                } else {
                    self.l2.set_public_ask_level(price, delta_pqty)?;
                    self.l2.set_current_ask_level(price, delta_cqty)?;
                }
                return Ok(());
            }

            // throw L2 price missing error if price is not found
            let current_pqty;
            let current_cqty;
            if is_bid {
                current_pqty = self.l2.public_bid_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
                current_cqty = self.l2.current_bid_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
            } else {
                current_pqty = self.l2.public_ask_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
                current_cqty = self.l2.current_ask_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
            }

            // Calculate new level quantity (add amount)
            let new_pqty = current_pqty.saturating_add(delta_pqty);
            let new_cqty = current_cqty.saturating_add(delta_cqty);

            if is_bid {
                self.l2.set_public_bid_level(price, new_pqty)?;
                self.l2.set_current_bid_level(price, new_cqty)?;
            } else {
                self.l2.set_public_ask_level(price, new_pqty)?;
                self.l2.set_current_ask_level(price, new_cqty)?;
            }

            // emit the event for the price level update on the orderbook
            event::emit_event(SpotEvent::SpotOrderBlockChanged {
                pair_id,
                is_bid,
                price,
                pqty: new_pqty,
                cqty: new_cqty,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
            });
            Ok(())
        } else {
            // Get current level quantity
            let current_cqty;
            let public_cqty;
            if is_bid {
                public_cqty = self.l2.public_bid_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
                current_cqty = self.l2.current_bid_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
            } else {
                public_cqty = self.l2.public_ask_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
                current_cqty = self.l2.current_ask_level(price).ok_or(OrderBookError::L2(
                    L2Error::PriceMissing {
                        price,
                        is_bid,
                        is_placed,
                    },
                ))?;
            };

            // Calculate new level quantity (subtract amount)
            // Use saturating_sub to prevent underflow, but we still check for 0
            let new_cqty = current_cqty.saturating_sub(delta_cqty);
            let new_pqty = public_cqty.saturating_sub(delta_pqty);

            // Update the level
            if is_bid {
                if new_cqty > 0 {
                    self.l2.set_public_bid_level(price, new_pqty)?;
                    self.l2.set_current_bid_level(price, new_cqty)?;
                } else {
                    // Level is 0 or below, remove the price
                    self.l2.set_public_bid_level(price, 0)?;
                    self.l2.set_current_bid_level(price, 0)?;
                    // Check if price level is empty in L3, and if so, remove it
                    if self.l3.is_empty(price) {
                        self.l2.remove_price(is_bid, price)?;
                    }
                }
            } else {
                if new_cqty > 0 {
                    self.l2.set_public_ask_level(price, new_pqty)?;
                    self.l2.set_current_ask_level(price, new_cqty)?;
                } else {
                    // Level is 0 or below, remove the price
                    self.l2.set_public_ask_level(price, 0)?;
                    self.l2.set_current_ask_level(price, 0)?;
                    // Check if price level is empty in L3, and if so, remove it
                    if self.l3.is_empty(price) {
                        self.l2.remove_price(is_bid, price)?;
                    }
                }
            }

            // If delete_price is Some, it means an order was fully consumed and price level was emptied
            // Remove that price level
            if let Some(delete_price) = delete_price {
                self.l2.remove_price(is_bid, delete_price)?;
            }

            // emit the event for the price level update on the orderbook
            event::emit_event(SpotEvent::SpotOrderBlockChanged {
                pair_id,
                is_bid,
                price,
                pqty: new_pqty,
                cqty: new_cqty,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
            });

            Ok(())
        }
    }

    /// Cancels an order.
    /// - returns the amount to send and the delete price.
    /// - `order_id` is the id of the order to cancel.
    /// - `owner` is the owner of the order.
    pub fn cancel_order(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        is_bid: bool,
        order_id: OrderId,
        owner: impl Into<Vec<u8>>,
    ) -> Result<(), OrderBookError> {
        let cid = cid.into();
        let pair_id = pair_id.into();
        let owner = owner.into();
        // check if the order exists
        let order = self.l3.get_order(order_id)?.clone();
        // check if the owner is the same as the owner of the order
        if order.owner != owner {
            return Err(OrderBookError::OrderNotOwnedBySender);
        }
        let deleted_price_opt = self.l3.delete_order(order_id)?;
        if let Some(price) = deleted_price_opt {
            self.l2.remove_price(is_bid, price)?;
        }

        // emit the event for the order cancelled
        event::emit_event(SpotEvent::SpotOrderCancelled {
            cid: cid.clone(),
            order_id: order_id.to_bytes().to_vec(),
            maker_account_id: order.owner.clone(),
            is_bid,
            price: order.price,
            amnt: order.amnt,
            iqty: order.iqty,
            pqty: order.pqty,
            cqty: order.cqty,
            timestamp: order.timestamp,
            expires_at: order.expires_at,
        });

        // update the price level on the orderbook
        self.update_price_level(
            pair_id.clone(),
            false,
            is_bid,
            order.price,
            order.pqty,
            order.cqty,
            deleted_price_opt,
        )?;
        Ok(())
    }

    pub fn expire_orders(
        &mut self,
        is_bid: bool,
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        managing_account_id: impl Into<Vec<u8>>,
        now: i64,
    ) -> Result<(), OrderBookError> {
        let pair_id = pair_id.into();
        let base_asset_id = base_asset_id.into();
        let quote_asset_id = quote_asset_id.into();
        let managing_account_id = managing_account_id.into();
        let expired_orders = self.l3.remove_dormant_orders(now);
        for (order_id, order) in expired_orders {
            // emit event for the order expired
            event::emit_event(SpotEvent::SpotOrderExpired {
                cid: order.cid.clone(),
                order_id: order_id.to_bytes().to_vec(),
                maker_account_id: order.owner.clone(),
                is_bid,
                price: order.price,
                amnt: order.amnt,
                iqty: order.iqty,
                pqty: order.pqty,
                cqty: order.cqty,
                timestamp: now,
                expires_at: order.expires_at,
            });
            // emit event for transfer of the expired asset to order owner
            let expired_asset_id = if is_bid {
                quote_asset_id.clone()
            } else {
                base_asset_id.clone()
            };
            event::emit_event(SpotEvent::Transfer {
                cid: order.cid.clone(),
                from: managing_account_id.clone(),
                to: order.owner.clone(),
                asset: expired_asset_id,
                amnt: order.amnt,
                timestamp: now,
            });

            // update the price level on the orderbook
            let delete_price = if self.l3.is_empty(order.price) {
                Some(order.price)
            } else {
                None
            };
            self.update_price_level(
                pair_id.clone(),
                false,
                is_bid,
                order.price,
                order.pqty,
                order.cqty,
                delete_price,
            )?;
        }
        Ok(())
    }

    pub fn set_iceberg_quantity(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        is_bid: bool,
        order_id: OrderId,
        iqty: u64,
    ) -> Result<(), OrderBookError> {
        let cid = cid.into();
        let pair_id = pair_id.into();
        let before = self.l3.get_order(order_id)?.clone();
        let order = self.l3.set_iceberg_quantity(order_id, iqty)?;
        let (is_placed, delta_pqty) = if order.pqty >= before.pqty {
            (true, order.pqty - before.pqty)
        } else {
            (false, before.pqty - order.pqty)
        };
        if delta_pqty > 0 {
            self.update_price_level(
                pair_id,
                is_placed,
                is_bid,
                order.price,
                delta_pqty,
                0,
                None,
            )?;
        }
        // emit event for the iceberg quantity changed
        event::emit_event(SpotEvent::SpotOrderIcebergQuantityChanged {
            cid,
            order_id: order_id.to_bytes().to_vec(),
            amnt: order.amnt,
            iqty: iqty,
            pqty: order.pqty,
            cqty: order.cqty,
            timestamp: order.timestamp,
            expires_at: order.expires_at,
        });
        Ok(())
    }
}
