use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::spot::Order;

use super::event::{self, SpotEvent};
use super::orderbook::{OrderBook, OrderBookError};
use super::orders::OrderId;
use super::time_in_force::TimeInForce;

use super::market::L1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Pair {
    /// Pair ID
    pub pair_id: Vec<u8>,
    /// base asset id
    pub base_asset_id: Vec<u8>,
    /// quote asset id
    pub quote_asset_id: Vec<u8>,
    /// L1 state
    pub l1: L1,
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
            pair_id: Vec::new(),
            base_asset_id: Vec::new(),
            quote_asset_id: Vec::new(),
            l1: L1::default(),
            orderbook: OrderBook::default(),
            clients: Vec::new(),
            client_admin_account_ids: HashMap::new(),
            client_fee_account_ids: HashMap::new(),
        }
    }

    pub fn add_client(
        &mut self,
        cid: impl Into<Vec<u8>>,
        admin_account_id: impl Into<Vec<u8>>,
        fee_account_id: impl Into<Vec<u8>>,
    ) {
        let cid = cid.into();
        let admin_account_id = admin_account_id.into();
        let fee_account_id = fee_account_id.into();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Store client and associated accounts
        self.clients.push(cid.clone());
        self.client_admin_account_ids
            .insert(cid.clone(), admin_account_id.clone());
        self.client_fee_account_ids
            .insert(cid.clone(), fee_account_id.clone());

        // Set up fee account for the orderbook
        self.orderbook
            .fee_recipients
            .insert(cid.clone(), fee_account_id.clone());

        // Emit event using the values we already have, avoiding extra lookups
        event::emit_event(SpotEvent::SpotPairClientAccountChanged {
            pair_id: self.pair_id.clone(),
            cid: Some(cid),
            admin_account_id: Some(admin_account_id),
            fee_account_id: Some(fee_account_id),
            timestamp,
        });
    }

    pub fn remove_client(&mut self, cid: impl Into<Vec<u8>>) {
        let cid = cid.into();

        // Remove from in-memory structures
        self.clients.retain(|c| *c != cid);
        self.client_admin_account_ids.remove(&cid);
        self.client_fee_account_ids.remove(&cid);
        // remove fee account from the orderbook
        self.orderbook.fee_recipients.remove(&cid);

        // Emit an event indicating the client was removed from this pair.
        // We keep `cid` so downstream consumers know which client changed,
        // and set admin/fee accounts to None to indicate removal.
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        event::emit_event(SpotEvent::SpotPairClientAccountChanged {
            pair_id: self.pair_id.clone(),
            cid: Some(cid),
            admin_account_id: None,
            fee_account_id: None,
            timestamp,
        });
    }
    
    /// Match all orders at a specific price level until the taker order is fully filled or no more orders at the price level
    /// Returns remaining_amount after matching
    /// `is_matching_asks` indicates if we're matching against ask orders (true) or bid orders (false)
    /// Continues matching until remaining amount is 0 or no more orders at the price level
    #[cfg_attr(test, allow(dead_code))]
    pub fn _match_at(
        &mut self,
        price: u64,
        is_matching_asks: bool,
        taker_order: &mut Order,
    ) -> Result<Order, OrderBookError> {
        let taker_id = taker_order.id;
        let mut current_remaining = taker_order.cqty;

        // Get the first order at this price level
        let mut maker_order_id = match self.orderbook.l3.head(price) {
            Some(id) => id,
            None => return Ok(taker_order.clone()), // No more orders
        };

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

            // Get maker order and refresh taker order before executing
            let maker_order = self.orderbook.l3.get_order(maker_order_id)?.clone();
            let taker_current = match self.orderbook.l3.get_order(taker_id) {
                Ok(order) => order.clone(),
                Err(_) => break,
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            self.orderbook.execute(
                taker_current,
                maker_order,
                self.pair_id.clone(),
                self.base_asset_id.clone(),
                self.quote_asset_id.clone(),
                now,
            )?;

            match self.orderbook.l3.get_order(taker_id) {
                Ok(updated) => current_remaining = updated.cqty,
                Err(_) => {
                    break;
                }
            }

            // traverse to the next order at the price level
            maker_order_id = match self.orderbook.l3.next(price, maker_order_id) {
                Some(id) => id,
                None => break, // No more orders
            };
        }

        let updated = match self.orderbook.l3.get_order(taker_id) {
            Ok(order) => order.clone(),
            Err(_) => {
                // if the taker order is not found, it means the taker order is fully filled
                let mut fallback = taker_order.clone();
                fallback.cqty = 0;
                fallback.pqty = 0;
                return Ok(fallback);
            }
        };
        Ok(updated)
    }

    /// Place a limit order (internal helper)
    /// Returns (remaining_amount, bid_head, ask_head)
    /// Continues matching until remaining amount is 0 or no more matching orders available
    #[cfg_attr(test, allow(dead_code))]
    pub fn _limit_order(
        &mut self,
        limit_price: u64,
        taker_order: &mut Order,
    ) -> Result<(Order, u64, u64), OrderBookError> {

        // Get last matched price
        let mut lmp = self.l1.lmp().unwrap_or(0);

        // Clear empty heads
        let mut bid_head = self.orderbook.clear_empty_head_or_zero(true);
        let mut ask_head = self.orderbook.clear_empty_head_or_zero(false);

        if taker_order.is_bid {
            // Limit Buy: match against ask orders
            if lmp != 0 {
                if ask_head != 0 && limit_price < ask_head {
                    return Ok((taker_order.clone(), bid_head, ask_head));
                } else if ask_head == 0 {
                    return Ok((taker_order.clone(), bid_head, ask_head));
                }
            }

            // Match against ask orders while ask_head <= limit_price
            let mut current_remaining = taker_order.cqty;
            while current_remaining > 0 && ask_head != 0 && ask_head <= limit_price {
                lmp = ask_head; // Update lmp to current match price
                let match_price = ask_head;

                // Match at this price level until remaining is 0 or price level is empty
                let updated = self._match_at(
                    match_price,
                    true, // matching against asks
                    taker_order,
                )?;
                *taker_order = updated;
                current_remaining = taker_order.cqty;

                // Update ask_head after matching (price level might be empty now)
                ask_head = self.orderbook.clear_empty_head_or_zero(false);
            }


            // Update bid_head
            bid_head = self.orderbook.clear_empty_head_or_zero(true);
        } else {
            // Limit Sell: match against bid orders
            if lmp != 0 {
                if bid_head != 0 && limit_price > bid_head {
                    return Ok((taker_order.clone(), bid_head, ask_head));
                } else if bid_head == 0 {
                    return Ok((taker_order.clone(), bid_head, ask_head));
                }
            }

            // Match against bid orders while bid_head >= limit_price
            let mut current_remaining = taker_order.cqty;
            while current_remaining > 0 && bid_head != 0 && bid_head >= limit_price {
                lmp = bid_head; // Update lmp to current match price
                let match_price = bid_head;

                // Match at this price level until remaining is 0 or price level is empty
                let updated = self._match_at(
                    match_price,
                    false, // matching against bids
                    taker_order,
                )?;
                *taker_order = updated;
                current_remaining = taker_order.cqty;

                // Update bid_head after matching (price level might be empty now)
                bid_head = self.orderbook.clear_empty_head_or_zero(true);
            }

            // Update ask_head
            ask_head = self.orderbook.clear_empty_head_or_zero(false);
        }

        // Set new market price if matches occurred
        if lmp != 0 {
            self.l1.set_lmp(lmp);
            // TODO: Emit NewMarketPrice event if we have such an event type
        }

        Ok((taker_order.clone(), bid_head, ask_head))
    }

    /// Handle time_in_force logic for an order after matching
    /// - `order_id`: The ID of the order to handle
    /// - `price`: The price of the order
    /// - `amount`: The original amount of the order
    /// - `remaining`: The remaining amount after matching
    /// - `is_bid`: Whether this is a bid order (true) or ask order (false)
    /// - `time_in_force`: The time in force policy
    /// Returns an error if FOK order is not fully filled
    fn _handle_time_in_force_post_matching(
        &mut self,
        time_in_force: TimeInForce,
        maker_order: &mut Order,
        is_limit_order: bool,
        maker_fee_bps: u16,
    ) -> Result<(), OrderBookError> {
        match time_in_force {
            TimeInForce::ImmediateOrCancel => {
                // IOC: Fill what can be filled immediately, cancel the rest
                if maker_order.cqty > 0 {
                    self.orderbook.cancel_order(maker_order.cid.clone(), self.pair_id.clone(), maker_order.is_bid, maker_order.id, maker_order.owner.clone())?;
                }
                Ok(())
            }
            TimeInForce::GoodTillCanceled => {
                // GTC: Place remaining in orderbook
                if maker_order.cqty > 0 {
                    // set the order's fee as maker fee basis points
                    maker_order.fee_bps = maker_fee_bps;

                    // determine the make price 
                    let make_price = if is_limit_order {
                        if maker_order.is_bid {
                            self.l1.det_limit_buy_make_price(maker_order.price, self.orderbook.l2.bid_head().unwrap_or(0), self.orderbook.l2.ask_head().unwrap_or(0), maker_fee_bps as u32)
                        } else {
                            self.l1.det_limit_sell_make_price(maker_order.price, self.orderbook.l2.bid_head().unwrap_or(0), self.orderbook.l2.ask_head().unwrap_or(0), maker_fee_bps as u32)
                        }
                    } else {
                        if maker_order.is_bid {
                            self.l1.det_market_buy_make_price(self.orderbook.l2.bid_head().unwrap_or(0), self.orderbook.l2.ask_head().unwrap_or(0), maker_fee_bps as u32)
                        } else {
                            self.l1.det_market_sell_make_price(self.orderbook.l2.bid_head().unwrap_or(0), self.orderbook.l2.ask_head().unwrap_or(0), maker_fee_bps as u32)
                        }
                    };
                } 
                Ok(())
            }
            _ => Err(OrderBookError::UnsupportedTimeInForce),
        }
    }

    fn can_fill_fok(&self, limit_price: u64, taker_order: &Order) -> Result<bool, OrderBookError> {
        let mut remaining = taker_order.cqty;

        let prices = if taker_order.is_bid {
            self.orderbook.l2.collect_ask_prices()
        } else {
            self.orderbook.l2.collect_bid_prices()
        };

        for price in prices {
            if taker_order.is_bid {
                if price > limit_price {
                    break;
                }
            } else if price < limit_price {
                break;
            }

            let level_cqty = if taker_order.is_bid {
                match self.orderbook.l2.current_ask_level(price) {
                    Some(level) => level,
                    None => continue,
                }
            } else {
                match self.orderbook.l2.current_bid_level(price) {
                    Some(level) => level,
                    None => continue,
                }
            };

            let required = self
                .orderbook
                .get_required(taker_order.clone(), price, level_cqty)?;

            if remaining <= required {
                return Ok(true);
            }

            remaining = remaining.saturating_sub(required);
            if remaining == 0 {
                return Ok(true);
            }
        }

        Ok(false)
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

        // place taker order to feed into _limit_order function
        let taker_order = self.orderbook.place_ask(
            cid_vec.clone(),
            self.pair_id.clone(),
            self.base_asset_id.clone(),
            self.quote_asset_id.clone(),
            owner_vec.clone(),
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            taker_fee_bps,
        )?;

        if matches!(time_in_force, TimeInForce::FillOrKill)
            && !self.can_fill_fok(price, &taker_order)?
        {
            self.orderbook.cancel_order(
                cid_vec.clone(),
                self.pair_id.clone(),
                false,
                taker_order.id,
                owner_vec.clone(),
            )?;
            return Err(OrderBookError::OrderNotFullyFilled);
        }
       
        // Match against existing orders FIRST (before placing in orderbook)
        let (taker_order, _bid_head, _ask_head) = self._limit_order(
            price,
            &mut taker_order.clone(),
        )?;

        // Handle time_in_force logic as maker order
        self._handle_time_in_force_post_matching(time_in_force, &mut taker_order.clone(), true, maker_fee_bps)?;

        Ok(taker_order.id)
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
    ) -> Result<(), OrderBookError> {

        let cid_vec: Vec<u8> = cid.into();
        let owner_vec: Vec<u8> = owner.into();
        if let Some(existing_order_id) = existing_order_id {
            let order = self.orderbook.l3.get_order(existing_order_id)?;
            if order.cid != cid_vec {
                return Err(OrderBookError::OrderNotSupportedByClientId);
            }
            if order.owner != owner_vec {
                return Err(OrderBookError::OrderNotOwnedBySender);
            }
        }

        let taker_order = self.orderbook.place_bid(
            cid_vec.clone(),
            self.pair_id.clone(),
            self.base_asset_id.clone(),
            self.quote_asset_id.clone(),
            owner_vec.clone(),
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            taker_fee_bps,
        )?;

        if matches!(time_in_force, TimeInForce::FillOrKill)
            && !self.can_fill_fok(price, &taker_order)?
        {
            self.orderbook.cancel_order(
                cid_vec.clone(),
                self.pair_id.clone(),
                true,
                taker_order.id,
                owner_vec.clone(),
            )?;
            return Err(OrderBookError::OrderNotFullyFilled);
        }

        let (taker_order, _bid_head, _ask_head) = self._limit_order(
            price,
            &mut taker_order.clone(),
        )?;

        self._handle_time_in_force_post_matching(time_in_force, &mut taker_order.clone(), true, maker_fee_bps)?;

        Ok(())
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
        // taker fee basis points of the order
        taker_fee_bps: u16,
        // time in force of the order
        time_in_force: TimeInForce,
    ) -> Result<(), OrderBookError> {

        let cid_vec: Vec<u8> = cid.into();
        let owner_vec: Vec<u8> = owner.into();
        // if existing order id is provided, update the order
        if let Some(existing_order_id) = existing_order_id {
            let order = self.orderbook.l3.get_order(existing_order_id)?;
            if order.cid != cid_vec {
                return Err(OrderBookError::OrderNotSupportedByClientId);
            }
            if order.owner != owner_vec {
                return Err(OrderBookError::OrderNotOwnedBySender);
            }
        }

        // get the best bid price
        let best_bid_price = self.orderbook.l2.bid_head();
        if best_bid_price.is_none() {
            return Err(OrderBookError::NoBidOrdersInOrderbook);
        }
        let price = best_bid_price.unwrap();

        let taker_order = self.orderbook.place_ask(
            cid_vec.clone(),
            self.pair_id.clone(),
            self.base_asset_id.clone(),
            self.quote_asset_id.clone(),
            owner_vec.clone(),
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            taker_fee_bps,
        )?;

        if matches!(time_in_force, TimeInForce::FillOrKill)
            && !self.can_fill_fok(price, &taker_order)?
        {
            self.orderbook.cancel_order(
                cid_vec.clone(),
                self.pair_id.clone(),
                false,
                taker_order.id,
                owner_vec.clone(),
            )?;
            return Err(OrderBookError::OrderNotFullyFilled);
        }
        
        let (taker_order, _bid_head, _ask_head) = self._limit_order(
            0,
            &mut taker_order.clone(),
        )?;

        self._handle_time_in_force_post_matching(time_in_force, &mut taker_order.clone(), false, maker_fee_bps)?;

        Ok(())
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
        // time in force of the order
        time_in_force: TimeInForce,
    ) -> Result<(), OrderBookError> {

        let cid_vec: Vec<u8> = cid.into();
        let owner_vec: Vec<u8> = owner.into();
        // if existing order id is provided, update the order
        if let Some(existing_order_id) = existing_order_id {
            let order = self.orderbook.l3.get_order(existing_order_id)?;
            if order.cid != cid_vec {
                return Err(OrderBookError::OrderNotSupportedByClientId);
            }
            if order.owner != owner_vec {
                return Err(OrderBookError::OrderNotOwnedBySender);
            }
        }

        // get the best ask price
        let best_ask_price = self.orderbook.l2.ask_head();
        if best_ask_price.is_none() {
            return Err(OrderBookError::NoAskOrdersInOrderbook);
        }
        let price = best_ask_price.unwrap();

        let taker_order = self.orderbook.place_bid(
            cid_vec.clone(),
            self.pair_id.clone(),
            self.base_asset_id.clone(),
            self.quote_asset_id.clone(),
            owner_vec.clone(),
            price,
            amnt,
            iqty,
            timestamp,
            expires_at,
            taker_fee_bps,
        )?;

        if matches!(time_in_force, TimeInForce::FillOrKill)
            && !self.can_fill_fok(price, &taker_order)?
        {
            self.orderbook.cancel_order(
                cid_vec.clone(),
                self.pair_id.clone(),
                true,
                taker_order.id,
                owner_vec.clone(),
            )?;
            return Err(OrderBookError::OrderNotFullyFilled);
        }

        let (taker_order, _bid_head, _ask_head) = self._limit_order(
            u64::MAX,
            &mut taker_order.clone(),
        )?;

        self._handle_time_in_force_post_matching(time_in_force, &mut taker_order.clone(), false,maker_fee_bps)?;

        Ok(())
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
            .cancel_order(cid, pair_id, is_bid, order_id, owner)?;
        Ok(())
    }
}
