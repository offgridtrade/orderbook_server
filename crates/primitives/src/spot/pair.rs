use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::event::{self, SpotEvent};
use super::orderbook::{OrderBook, OrderBookError, OrderMatch};
use super::orders::OrderId;
use super::time_in_force::TimeInForce;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Pair {
    /// Pair ID
    pub pair_id: String,
    /// base asset id
    pub base_asset_id: Vec<u8>,
    /// quote asset id
    pub quote_asset_id: Vec<u8>,
    /// Orderbook
    pub orderbook: OrderBook,
    /// list of exchange clients which shares the orderbook 
    pub clients: Vec<Vec<u8>>,
    /// Hash map of client id -> client admin account id
    pub client_admin_account_ids: HashMap<Vec<u8>, Vec<u8>>,
    /// Hash map of client id -> client fee account id
    pub client_fee_account_ids: HashMap<Vec<u8>, Vec<u8>>,
}

impl Pair {

    pub fn new() -> Self {
        Self {
            pair_id: String::new(),
            base_asset_id: Vec::new(),
            quote_asset_id: Vec::new(),
            orderbook: OrderBook::default(),
            clients: Vec::new(),
            client_admin_account_ids: HashMap::new(),
            client_fee_account_ids: HashMap::new(),
        }
    }

    pub fn add_client(&mut self, cid: impl Into<Vec<u8>>, admin_account_id: impl Into<Vec<u8>>, fee_account_id: impl Into<Vec<u8>>) {
        let cid = cid.into();
        let admin_account_id = admin_account_id.into();
        let fee_account_id = fee_account_id.into();
        self.clients.push(cid.clone());
        self.client_admin_account_ids.insert(cid.clone(), admin_account_id);
        self.client_fee_account_ids.insert(cid, fee_account_id);
    }

    pub fn remove_client(&mut self, cid: impl Into<Vec<u8>>) {
        let cid = cid.into();
        self.clients.retain(|c| *c != cid);
        self.client_admin_account_ids.remove(&cid);
        self.client_fee_account_ids.remove(&cid);
    }
    
    /// Match orders at a specific price level
    /// Returns remaining_amount after matching
    /// `is_matching_asks` indicates if we're matching against ask orders (true) or bid orders (false)
    /// Continues matching until remaining amount is 0 or no more orders at the price level
    #[cfg_attr(test, allow(dead_code))]
    pub fn _match_at(
        &mut self,
        cid: Vec<u8>,
        price: u64,
        remaining: u64,
        taker_fee_bps: u16,
        is_matching_asks: bool,
        taker_account_id: Vec<u8>,
    ) -> Result<u64, OrderBookError> {
        let mut current_remaining = remaining;

        // Keep matching until remaining is 0 or price level is empty
        while current_remaining > 0 {
            // Check if price level is empty
            if self.orderbook.l3.is_empty(price) {
                // Remove price level: if matching asks, price level is ask (is_bid = false)
                // if matching bids, price level is bid (is_bid = true)
                let is_bid = !is_matching_asks;
                self.orderbook.l2.remove_price(is_bid, price)?;
                break;
            }

            // Get the first order at this price level
            let maker_order_id = match self.orderbook.l3.head(price) {
                Some(id) => id,
                None => break, // No more orders
            };

            // Get maker order to check its amount
            let match_amount = {
                let maker_order = self.orderbook.l3.get_order(maker_order_id)?;
                // Calculate match amount (minimum of remaining and maker order's current quantity)
                current_remaining.min(maker_order.cqty)
            };

            // Execute the match by decreasing the maker order only
            // The taker order will be handled by the caller (limit_buy/limit_sell)
            let (_, delete_price) = self.orderbook.l3.decrease_order(
                maker_order_id,
                match_amount,
                1u64,
                false, // clear only if fully matched
            )?;

            current_remaining -= match_amount;

            // If maker order was fully consumed and price level became empty
            if delete_price.is_some() && self.orderbook.l3.is_empty(price) {
                // Remove price level
                let is_bid = !is_matching_asks;
                self.orderbook.l2.remove_price(is_bid, price)?;
                break;
            }
        }

        Ok(current_remaining)
    }

    /// Place a limit order (internal helper)
    /// Returns (remaining_amount, bid_head, ask_head)
    /// Continues matching until remaining amount is 0 or no more matching orders available
    #[cfg_attr(test, allow(dead_code))]
    pub fn _limit_order(
        &mut self,
        cid: Vec<u8>,
        amount: u64,
        is_bid: bool,
        limit_price: u64,
        taker_fee_bps: u16,
        taker_account_id: Vec<u8>,
    ) -> Result<(u64, u64, u64), OrderBookError> {
        let mut remaining = amount;

        // Get last matched price
        let mut lmp = self.orderbook.lmp().unwrap_or(0);

        // Clear empty heads
        let mut bid_head = self.orderbook.clear_empty_head_or_zero(true);
        let mut ask_head = self.orderbook.clear_empty_head_or_zero(false);

        if is_bid {
            // Limit Buy: match against ask orders
            if lmp != 0 {
                if ask_head != 0 && limit_price < ask_head {
                    return Ok((remaining, bid_head, ask_head));
                } else if ask_head == 0 {
                    return Ok((remaining, bid_head, ask_head));
                }
            }

            // Match against ask orders while ask_head <= limit_price
            while remaining > 0 && ask_head != 0 && ask_head <= limit_price {
                lmp = ask_head; // Update lmp to current match price
                let match_price = ask_head;

                // Match at this price level until remaining is 0 or price level is empty
                remaining = self._match_at(
                    cid.clone(),
                    match_price,
                    remaining,
                    taker_fee_bps,
                    true, // matching against asks
                    taker_account_id.clone(),
                )?;

                // Update ask_head after matching (price level might be empty now)
                ask_head = self.orderbook.clear_empty_head_or_zero(false);
            }

            // Update bid_head
            bid_head = self.orderbook.clear_empty_head_or_zero(true);
        } else {
            // Limit Sell: match against bid orders
            if lmp != 0 {
                if bid_head != 0 && limit_price > bid_head {
                    return Ok((remaining, bid_head, ask_head));
                } else if bid_head == 0 {
                    return Ok((remaining, bid_head, ask_head));
                }
            }

            // Match against bid orders while bid_head >= limit_price
            while remaining > 0 && bid_head != 0 && bid_head >= limit_price {
                lmp = bid_head; // Update lmp to current match price
                let match_price = bid_head;

                // Match at this price level until remaining is 0 or price level is empty
                remaining = self._match_at(
                    cid.clone(),
                    match_price,
                    remaining,
                    taker_fee_bps,
                    false, // matching against bids
                    taker_account_id.clone(),
                )?;

                // Update bid_head after matching (price level might be empty now)
                bid_head = self.orderbook.clear_empty_head_or_zero(true);
            }

            // Update ask_head
            ask_head = self.orderbook.clear_empty_head_or_zero(false);
        }

        // Set new market price if matches occurred
        if lmp != 0 {
            self.orderbook.set_lmp(lmp);
            // TODO: Emit NewMarketPrice event if we have such an event type
        }

        Ok((remaining, bid_head, ask_head))
    }

    /// Handle time_in_force logic for an order after matching
    /// - `order_id`: The ID of the order to handle
    /// - `price`: The price of the order
    /// - `amount`: The original amount of the order
    /// - `remaining`: The remaining amount after matching
    /// - `is_bid`: Whether this is a bid order (true) or ask order (false)
    /// - `time_in_force`: The time in force policy
    /// Returns an error if FOK order is not fully filled
    fn _handle_time_in_force(
        &mut self,
        order_id: OrderId,
        price: u64,
        amount: u64,
        remaining: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
    ) -> Result<(), OrderBookError> {
        match time_in_force {
            TimeInForce::FillOrKill => {
                // FOK: Must be fully filled immediately, otherwise cancel
                if remaining > 0 {
                    // Not fully filled, cancel the order
                    self.orderbook.l3.delete_order(order_id)?;
                    return Err(OrderBookError::AmountIsZero); // Or create a specific error for FOK rejection
                }
                // Fully matched, remove the order
                self.orderbook.l3.delete_order(order_id)?;
            }
            TimeInForce::ImmediateOrCancel => {
                // IOC: Fill what can be filled immediately, cancel the rest
                if remaining > 0 {
                    // Partially filled, update order and cancel remaining
                    let matched = amount - remaining;
                    if matched > 0 {
                        self.orderbook
                            .l3
                            .decrease_order(order_id, matched, 1u64, false)?;
                    }
                    // Cancel remaining portion (delete order)
                    self.orderbook.l3.delete_order(order_id)?;
                } else {
                    // Fully matched, remove the order
                    self.orderbook.l3.delete_order(order_id)?;
                }
            }
            TimeInForce::GoodTillCanceled => {
                // GTC: Place remaining in orderbook
                if remaining > 0 {
                    // Update order quantity to remaining amount
                    let matched = amount - remaining;
                    if matched > 0 {
                        self.orderbook
                            .l3
                            .decrease_order(order_id, matched, 1u64, false)?;
                    }

                    // Insert into orderbook
                    self.orderbook
                        .l3
                        .insert_id(price, order_id, remaining as u128)?;
                    self.orderbook.l2.insert_price(is_bid, price)?;
                } else {
                    // Fully matched, remove the order
                    self.orderbook.l3.delete_order(order_id)?;
                }
            }
        }
        Ok(())
    }

    /// Matches against existing orders first, then places remaining in orderbook based on time_in_force
    /// - returns the order id and if a dormant order was found.
    /// - `cid` is the gateway client id.
    /// - `existing_order_id` is the order id to update with the transaction if it exists.
    /// - `owner` is the owner of the order.
    /// - `price` is the price of the order.
    /// - `amount` is the total amount of the order.
    /// - `public_amount` is the public amount of the order in case of iceberg order.
    /// - `timestamp` is the timestamp of the order.
    /// - `expires_at` is the expiring timestamp of the order.
    /// - `maker_fee_bps` is the maker fee basis points of the order.
    /// - `taker_fee_bps` is the taker fee basis points of the order.
    /// - `time_in_force` is the time in force of the order.
    pub fn limit_sell(
        &mut self,
        // gateway client id
        cid: impl Into<Vec<u8>>,
        // order id to update with the transaction if it exists
        existing_order_id: Option<OrderId>, // None if new order
        // owner of the order
        owner: impl Into<Vec<u8>>,
        // price of the order
        price: u64,
        // total amount of the order
        amnt: u64,
        // iceberg quantity of the order
        iqty: u64,
        // timestamp of the order
        timestamp: i64,
        // expiring timestamp of the order
        expires_at: i64,
        // maker fee basis points of the order
        maker_fee_bps: u16,
        // taker fee basis points of the order
        taker_fee_bps: u16,
        // time in force of the order
        time_in_force: TimeInForce,
    ) -> Result<OrderId, OrderBookError> {
        // If existing order id is provided, update the order
        let cid_vec: Vec<u8> = cid.into();
        let owner_vec: Vec<u8> = owner.into();
        if let Some(existing_order_id) = existing_order_id {
            let order = self.orderbook.l3.get_order(existing_order_id)?;
            if order.cid != cid_vec {
                return Err(OrderBookError::OrderNotOwnedBySender);
            }
        }
       
        // Match against existing orders FIRST (before placing in orderbook)
        let (remaining, _bid_head, _ask_head) = self._limit_order(
            cid_vec.clone(),
            amnt,
            false, // is_bid = false for sell
            price,
            taker_fee_bps,
            owner_vec.clone(),
        )?;

        // Create the order (but don't insert into orderbook yet)
        let order_id = self.orderbook.place_ask(
            cid_vec.clone(),
            self.pair_id.as_bytes().to_vec(),
            owner_vec.clone(),
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            maker_fee_bps,
        )?;

        // Handle time_in_force logic
        self._handle_time_in_force(order_id, price, amnt, remaining, false, time_in_force)?;

        // Emit OrderPlaced event
        // TODO: emit the event inside the _handle_time_in_force function
        let order_info = self.orderbook.l3.get_order(order_id);
        if let Ok(order) = order_info {
            event::emit_event(SpotEvent::SpotOrderPlaced {
                cid: cid_vec.clone(),
                order_id: order_id.to_bytes().to_vec(),
                maker_account_id: order.owner.clone(), // Vec<u8>
                is_bid: false, // ask order
                price: order.price,
                amnt: order.amnt,
                iqty: order.iqty,
                pqty: order.pqty,
                cqty: order.cqty,
                timestamp: order.timestamp,
                expires_at: order.expires_at,
            });
        }

        Ok(order_id)
    }

    /// Place a limit buy order (bid order)
    /// Matches against existing orders first, then places remaining in orderbook based on time_in_force
    /// - returns the order id and if a dormant order was found.
    /// - `cid` is the gateway client id.
    /// - `existing_order_id` is the order id to update with the transaction if it exists.
    /// - `owner` is the owner of the order.
    /// - `price` is the price of the order.
    /// - `amount` is the total amount of the order.
    /// - `public_amount` is the public amount of the order in case of iceberg order.
    /// - `timestamp` is the timestamp of the order.
    /// - `expires_at` is the expiring timestamp of the order.
    /// - `maker_fee_bps` is the maker fee basis points of the order.
    /// - `taker_fee_bps` is the taker fee basis points of the order.
    /// - `time_in_force` is the time in force of the order.
    pub fn limit_buy(
        &mut self,
        // gateway client id
        cid: impl Into<Vec<u8>>,
        // order id to update with the transaction if it exists
        existing_order_id: Option<OrderId>, // None if new order
        // owner of the order
        owner: impl Into<Vec<u8>>,
        // price of the order
        price: u64,
        // total amount of the order
        amnt: u64,
        // iceberg quantity of the order
        iqty: u64,
        // timestamp of the order
        timestamp: i64,
        // expiring timestamp of the order
        expires_at: i64,
        // maker fee basis points of the order
        maker_fee_bps: u16,
        // taker fee basis points of the order
        taker_fee_bps: u16,
        // time in force of the order
        time_in_force: TimeInForce,
    ) -> Result<OrderId, OrderBookError> {

        Ok(existing_order_id.unwrap_or_else(OrderId::new))
    }

    /// Execute a market sell order
    /// Matches against existing orders first (market orders match at any price)
    /// - returns the match result.
    /// - `cid` is the gateway client id.
    /// - `existing_order_id` is the order id to update with the transaction if it exists.
    /// - `owner` is the owner of the order.
    /// - `amount` is the total amount of the order.
    /// - `clear` is whether to clear the order.
    /// - `taker_fee_bps` is the taker fee basis points of the order.
    /// - `time_in_force` is the time in force of the order.
    pub fn market_sell(
        &mut self,
        // gateway client id
        cid: impl Into<Vec<u8>>,
        // existing order id to update with the transaction if it exists
        existing_order_id: Option<OrderId>, // None if new order
        // owner of the order
        owner: impl Into<Vec<u8>>,
        // total amount of the order
        amnt: u64,
        // iceberg quantity of the order
        iqty: u64,
        // timestamp of the order
        timestamp: i64,
        // expiring timestamp of the order
        expires_at: i64,
        // maker fee basis points of the order
        maker_fee_bps: u16,
        taker_fee_bps: u16,
        time_in_force: TimeInForce,
    ) -> Result<OrderMatch, OrderBookError> {
        
        let taker_account_id = owner.into();
        let managing_account_id = Vec::new();
        let clear = false;
        let order_id = existing_order_id.unwrap_or_else(OrderId::new);
        let matched = amnt;
        self.orderbook
            .execute(
                false,
                order_id,
                self.pair_id.as_bytes().to_vec(),
                self.base_asset_id.clone(),
                self.quote_asset_id.clone(),
                taker_account_id,
                managing_account_id,
                matched,
                clear,
                timestamp,
                taker_fee_bps,
            )
    }

    /// Execute a market buy order
    /// Matches against existing orders first (market orders match at any price)
    /// - returns the match result.
    /// - `cid` is the gateway client id.
    /// - `existing_order_id` is the order id to update with the transaction if it exists.
    /// - `owner` is the owner of the order.
    /// - `amount` is the total amount of the order.
    /// - `taker_fee_bps` is the taker fee basis points of the order.
    /// - `time_in_force` is the time in force of the order.
    pub fn market_buy(
        &mut self,
        // gateway client id
        cid: impl Into<Vec<u8>>,
        // existing order id to update with the transaction if it exists
        existing_order_id: Option<OrderId>, // None if new order
        // owner of the order
        owner: impl Into<Vec<u8>>,
        // total amount of the order
        amnt: u64,
        // iceberg quantity of the order
        iqty: u64,
        // timestamp of the order
        timestamp: i64,
        // expiring timestamp of the order
        expires_at: i64,
        // maker fee basis points of the order
        maker_fee_bps: u16,
        // taker fee basis points of the order
        taker_fee_bps: u16,
        // time in forcw
        time_in_force: TimeInForce,
    ) -> Result<OrderMatch, OrderBookError> {

        let taker_account_id = owner.into();
        let managing_account_id = Vec::new();
        let order_id = existing_order_id.unwrap_or_else(OrderId::new);
        let clear = false;
        let matched = amnt;
        self.orderbook
            .execute(
                true,
                order_id,
                self.pair_id.as_bytes().to_vec(),
                self.base_asset_id.clone(),
                self.quote_asset_id.clone(),
                taker_account_id,
                managing_account_id,
                matched,
                false,
                timestamp,
                taker_fee_bps,
            )
    }

    pub fn cancel_order(
        &mut self,
        cid: impl Into<Vec<u8>>,
        pair_id: impl Into<Vec<u8>>,
        is_bid: bool,
        order_id: OrderId,
        owner: impl Into<Vec<u8>>,
    ) -> Result<(), OrderBookError> {
        self.orderbook
            .cancel_order(cid, pair_id, is_bid, order_id, owner)
    }
}
