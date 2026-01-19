use serde::{Deserialize, Serialize};

use crate::spot::{Order, event::{self, SpotEvent}};

use super::{
    market::L1Error,
    orders::{L3Error, OrderId},
    prices::L2Error,
    L1, L2, L3,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]

pub struct OrderMatch {
    pub sender: Vec<u8>,
    pub owner: Vec<u8>,
    pub base_amount: u64,
    pub quote_amount: u64,
    pub base_fee: u64,
    pub quote_fee: u64,
    pub trade_id: OrderId,
}

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
    #[error("iceberg quantity is bigger than whole amount")]
    IcebergQuantityIsBiggerThanWholeAmount,
    #[error("order has expired")]
    OrderExpired,
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

    /// Gets the last matched price (lmp)
    pub fn lmp(&self) -> Option<u64> {
        self.l1.lmp
    }

    /// Sets the last matched price (lmp)
    pub fn set_lmp(&mut self, price: u64) {
        self.l1.lmp = Some(price);
    }

    /// Gets the required amount of base liquidity to match an order and clear the order.
    pub fn get_required(&self, order_id: OrderId) -> Result<u64, OrderBookError> {
        let order = self.l3.get_order(order_id)?;
        Ok(order.cqty)
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
                return Ok(order.expect("head price must have at least one order").clone());
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
        self.update_price_level(cid, pair_id, true, true, price, pqty, amnt, None)?;
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
        self.update_price_level(cid, pair_id, true, false, price, pqty, amnt, None)?;
        Ok(order)
    }

    /// Executes a trade.
    /// - returns the trade details.
    /// - `is_bid` is whether the order is a bid order.
    /// - `order_id` is the id of the order to match against in the orderbook.
    /// - `amount` is the amount of the order.
    /// - `clear` is whether to clear the order.
    /// - `now` is the current timestamp used for expiration checks.
    /// - `taker_fee_bps` is the taker fee basis points of the order.
    /// when the order is cleared, the price level is updated on bid and ask side.
    /// The function returns OrderMatch to report events
    pub fn execute(
        &mut self,
        cid: impl Into<Vec<u8>>,
        taker_order: Order,
        maker_order: Order,
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        is_bid: bool,
        taker_account_id: impl Into<Vec<u8>>,
        managing_account_id: impl Into<Vec<u8>>,
        amount: u64,
        clear: bool,
        now: i64,
        taker_fee_bps: u16,
    ) -> Result<OrderMatch, OrderBookError> {
        // Get order data before mutable borrow
        if maker_order.expires_at <= now {
            self._expire_order(maker_order.id, is_bid, pair_id.into(), now)?;
            // let _match_at at pair.rs handle the expired order error
            return Err(OrderBookError::OrderExpired);
        }

        let (amount_to_send, delete_price) =
            self.l3
                .decrease_order(maker_order.id, amount, 1u64, clear)?;

        // Calculate base and quote amounts based on order type
        let (base_amount, quote_amount) = if is_bid {
            // For bid orders: amount_to_send is quote currency, calculate base received
            let base = (amount_to_send * 1_0000_0000) / maker_order.price;
            (base, amount_to_send)
        } else {
            // For ask orders: amount_to_send is base currency, calculate quote received
            let quote = (amount_to_send * maker_order.price) / 1_0000_0000;
            (amount_to_send, quote)
        };

        // Calculate fees using fee table
        let (base_fee, quote_fee) = self._calculate_fees(
            is_bid,
            base_amount,
            quote_amount,
            maker_order.maker_fee_bps,
            taker_fee_bps,
        );

        // emit the event for order matched
        let match_timestamp = now;

        // emit event for both maker and taker
        let (remaining_cqty, remaining_pqty) = match self.l3.get_order(maker_order.id) {
            Ok(updated) => (updated.cqty, updated.pqty),
            Err(_) => (0, 0),
        };
        let delta_cqty = maker_order.cqty.saturating_sub(remaining_cqty);
        let delta_pqty = maker_order.pqty.saturating_sub(remaining_pqty);
        // emit maker / taker order history event
        self._emit_taker_maker_match(
            taker_order_cid,
            taker_order_id,
            maker_order.cid.clone(),
            maker_order_id,
            taker_account_id.clone(),
            
            order_owner.clone(),
            is_bid,
            order_price,
            pair_id.clone(),
            base_asset_id.clone(),
            quote_asset_id.clone(),
            base_amount,
            quote_amount,
            base_fee,
            quote_fee,
            match_timestamp,
            order_expires_at,
        )?;
        
        // emit the event for sending fees from maker and taker to the managing account
        self._emit_fee_transfers(
            order_cid.clone(),
            is_bid,
            order_owner.clone(),
            taker_account_id,
            managing_account_id,
            base_asset_id,
            quote_asset_id,
            base_fee,
            quote_fee,
        );

        // adjust price level on the matched amount
        // Update levels and remove price if level becomes 0 or below
        // Also handle delete_price removal if an order was fully consumed
        self.update_price_level(
            order_cid,
            pair_id,
            false,
            is_bid,
            order_price,
            delta_pqty,
            delta_cqty,
            delete_price,
        )?;

        Ok(OrderMatch {
            sender: order_owner.clone(),
            owner: order_owner,
            base_amount,
            quote_amount,
            base_fee,
            quote_fee,
            trade_id: maker_order_id,
        })
    }

    fn _calculate_fees(
        &self,
        is_bid: bool,
        base_amount: u64,
        quote_amount: u64,
        maker_fee_bps: u16,
        taker_fee_bps: u16,
    ) -> (u64, u64) {
        // find maker and taker from base and quote amount
        if is_bid {
            (
                base_amount * maker_fee_bps as u64 / 10000,
                quote_amount * taker_fee_bps as u64 / 10000,
            )
        } else {
            (
                base_amount * taker_fee_bps as u64 / 10000,
                quote_amount * maker_fee_bps as u64 / 10000,
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
            order.cid.clone(),
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
        taker_account_id: impl Into<Vec<u8>>,
        maker_account_id: impl Into<Vec<u8>>,
        taker_matching_cqty: u64,
        maker_remaining_cqty: u64,
        is_bid: bool,
        price: u64,
        pair_id: impl Into<Vec<u8>>,
        base_asset_id: impl Into<Vec<u8>>,
        quote_asset_id: impl Into<Vec<u8>>,
        base_amount: u64,
        quote_amount: u64,
        base_fee: u64,
        quote_fee: u64,
        timestamp: i64,
        expires_at: i64,
    ) -> Result<(), OrderBookError> {
        Ok(())
    }

    fn _emit_fee_transfers(
        &self,
        cid: Vec<u8>,
        is_bid: bool,
        maker_account_id: Vec<u8>,
        taker_account_id: Vec<u8>,
        managing_account_id: Vec<u8>,
        base_asset_id: Vec<u8>,
        quote_asset_id: Vec<u8>,
        base_fee: u64,
        quote_fee: u64,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let (base_fee_from, quote_fee_from) = if is_bid {
            (maker_account_id, taker_account_id)
        } else {
            (taker_account_id, maker_account_id)
        };

        event::emit_event(SpotEvent::Transfer {
            cid: cid.clone(),
            from: base_fee_from,
            to: managing_account_id.clone(),
            asset: base_asset_id,
            amnt: base_fee,
            timestamp,
        });

        event::emit_event(SpotEvent::Transfer {
            cid,
            from: quote_fee_from,
            to: managing_account_id,
            asset: quote_asset_id,
            amnt: quote_fee,
            timestamp,
        });
    }

    /// updates the levels on the orderbook in the price level linked list
    /// - `is_bid` is whether the order is a bid order.
    /// - `price` is the price of the order.
    /// - `amount` is the amount to add to the level when isPlaced is true, or the amount to subtract from the level when isPlaced is false.
    /// - `delete_price` is an optional price that should be removed (when an order was fully consumed).
    /// Removes the price if the level becomes 0 or below.
    pub fn update_price_level(
        &mut self,
        cid: Vec<u8>,
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
                cid,
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
                cid,
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
            cid,
            pair_id,
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
                order.cid.clone(),
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
                cid.clone(),
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
