use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Represents an order stored in the order book.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Order {
    /// client order id
    pub cid: Vec<u8>,
    /// owner of the order
    pub owner: Vec<u8>,
    /// price of the order in 8 decimals
    pub price: u64,
    /// public liquidity for iceberg orders to protect position in 8 decimals
    pub pq: u64,
    /// initial liquidity of the order in 8 decimals
    pub iq: u64,
    /// current liquidity of the order in 8 decimals
    pub cq: u64,
    /// timestamp of the order in milliseconds
    pub timestamp: i64,
}

impl Order {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cid: Vec<u8>,
        owner: Vec<u8>,
        price: u64,
        pq: u64,
        iq: u64,
        cq: u64,
        timestamp: i64,
    ) -> Self {
        Self {
            cid,
            owner,
            price,
            pq,
            iq,
            cq,
            timestamp,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum L3Error {
    #[error("order id is zero: {0}")]
    OrderIdIsZero(u32),
    #[error("price is zero")]
    PriceIsZero,
    #[error("order does not exist: {0}")]
    OrderDoesNotExist(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct L3 {
    /// Mapping price -> linked list stored as `current -> next`.
    pub order_list: BTreeMap<u64, BTreeMap<u32, u32>>,
    /// Mapping price -> head of the linked list.
    pub order_head: BTreeMap<u64, u32>,
    /// Mapping price -> last of the linked list.
    pub order_tail: BTreeMap<u64, u32>,
    /// Order details keyed by id. Mapping order_id -> Order.
    pub orders: BTreeMap<u32, Order>,
    /// Sequential counter for new order ids.
    pub count: u32,
    /// dust limit to determine if the order should be deleted
    pub dust: u64,
    /// Last displaced order when IDs collide.
    pub dormant_order: Option<Order>,
}

impl L3 {
    pub fn new() -> Self {
        Self {
            order_list: BTreeMap::new(),
            order_head: BTreeMap::new(),
            order_tail: BTreeMap::new(),
            orders: BTreeMap::new(),
            count: 0,
            dust: 1,
            dormant_order: None,
        }
    }

    fn ensure_price(price: u64) -> Result<(), L3Error> {
        if price == 0 {
            Err(L3Error::PriceIsZero)
        } else {
            Ok(())
        }
    }

    /// Sets the dust limit to determine if the order should be deleted
    pub fn set_dust(&mut self, dust: u64) {
        self.dust = dust;
    }

    /// Inserts an order id into the linked structure for a given price level,
    /// keeping FIFO (append at tail).
    pub fn insert_id(&mut self, price: u64, id: u32, _amount: u128) -> Result<(), L3Error> {
        Self::ensure_price(price)?;

        let level = self.order_list.entry(price).or_insert_with(BTreeMap::new);
        let last_id = self.order_tail.get(&price).copied().unwrap_or(0);

        if last_id != 0 {
            level.insert(last_id, id);
        } else {
            self.order_head.insert(price, id);
        }

        level.insert(id, 0);
        self.order_tail.insert(price, id);

        Ok(())
    }

    /// Removes and returns the first order id at the given price level.
    /// returns (order_id, is_empty)
    /// - `order_id` is the id of the first order in the price level.
    /// - `is_empty` is true when the price level becomes empty.
    pub fn pop_front(&mut self, price: u64) -> Result<(Option<u32>, bool), L3Error> {
        Self::ensure_price(price)?;
        let head_id = self.order_head.get(&price).copied().unwrap_or(0);
        if head_id == 0 {
            return Ok((None, true));
        }
        let next = self
            .order_list
            .get_mut(&price)
            .and_then(|level| level.remove(&head_id))
            .unwrap_or(0);

        if next != 0 {
            self.order_head.insert(price, next);
            Ok((Some(head_id), false))
        } else {
            self.order_head.remove(&price);
            self.order_tail.remove(&price);
            self.order_list.remove(&price);
            return Ok((Some(head_id), true));
        }
    }

    /// Creates a new order, assigning the next id. Returns `(id, found_dormant)`.
    /// - `id` is the id of the new order.
    /// - `found_dormant` is true when a dormant order was found and stored in the `dormant_order` field.
    pub fn create_order(
        &mut self,
        cid: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        price: u64,
        iq: u64,
        pq: u64,
        timestamp: i64,
    ) -> Result<(u32, bool), L3Error> {
        Self::ensure_price(price)?;
        let cid = cid.into();
        let owner = owner.into();
        let order = Order::new(cid, owner, price, pq, iq, iq, timestamp);

        self.count = if self.count == 0 || self.count == u32::MAX {
            1
        } else {
            self.count.saturating_add(1)
        };

        if let Some(existing) = self.orders.get(&self.count).cloned() {
            self.dormant_order = Some(existing);
            self.delete_order(self.count)?;
            self.orders.insert(self.count, order);
            return Ok((self.count, true));
        }

        self.orders.insert(self.count, order);
        Ok((self.count, false))
    }

    /// Decreases the deposit amount for a given order id.
    ///
    /// Returns `(amount_to_send, deleted_price_if_level_empty)` where:
    /// - `amount_to_send` is the liquidity that should be returned to the caller. based on the dust limit and the clear flag.
    /// - `deleted_price_if_level_empty` is `Some(price)` when the price level becomes empty.
    pub fn decrease_order(
        &mut self,
        id: u32,
        amount: u64,
        dust: u64,
        clear: bool,
    ) -> Result<(u64, Option<u64>), L3Error> {
        if id == 0 {
            return Err(L3Error::OrderIdIsZero(id));
        }

        let mut amount_to_send;
        let mut should_delete = false;

        {
            let order = match self.orders.get_mut(&id) {
                Some(order) => order,
                None => return Ok((0, None)),
            };

            let original = order.cq;
            amount_to_send = amount.min(original);
            let decreased = original.saturating_sub(amount_to_send);

            if clear || decreased <= dust {
                amount_to_send = original;
                should_delete = true;
            } else {
                order.cq = decreased;
            }
        }

        if should_delete {
            let delete_price = self.delete_order(id)?;
            Ok((amount_to_send, delete_price))
        } else {
            Ok((amount_to_send, None))
        }
    }

    /// Deletes an order from the storage, returning the price level if it becomes empty.
    /// - returns the price level if it becomes empty.
    pub fn delete_order(&mut self, id: u32) -> Result<Option<u64>, L3Error> {
        if id == 0 {
            return Err(L3Error::OrderIdIsZero(id));
        }

        // remove order id from the list of orders
        let price = self
            .orders
            .get(&id)
            .ok_or(L3Error::OrderDoesNotExist(id))?
            .price;
        let head_id = self.order_head.get(&price).copied();
        // if the id to delete is at the head of the price level order list
        if head_id == Some(id) {
            // remove the id from the price level order list
            let next = self
                .order_list
                .get_mut(&price)
                .and_then(|lvl| lvl.remove(&id))
                .unwrap_or(0);
            // if the next is not 0, the next is the new head
            if next != 0 {
                // insert the next in the head position
                self.order_head.insert(price, next);
            }
            // if the next is 0, the price level becomes empty
            else {
                self.order_head.remove(&price);
                self.order_tail.remove(&price);
                self.order_list.remove(&price);
                // remove order from the orders map
                self.orders.remove(&id);
                return Ok(Some(price));
            }
        }
        // if the id to delete is not at the head of the price level order list
        else if let Some(level) = self.order_list.get_mut(&price) {
            let mut current = head_id;
            while current.is_some() {
                let next = level.get(&current.unwrap());
                if let Some(&next_id) = next {
                    // if the next is the id to delete, remove the id from the price level order list
                    if next_id == id {
                        let next_next = level.get(&next_id).copied().unwrap_or(0);
                        // if the next next is 0, the next is the new tail
                        if next_next == 0 {
                            // if the next next is 0, the current is the new tail
                            self.order_tail.insert(price, current.unwrap());
                        }
                        // insert the next next in the current position
                        level.insert(current.unwrap(), next_next);
                        break;
                    }
                }
                // if the next is not the id to delete, move to the next
                current = next.copied();
            }
        }
        // remove order from the orders map
        self.orders.remove(&id);
        Ok(None)
    }

    /// Returns the next id that would be assigned on order creation.
    pub fn next_make_id(&self) -> u32 {
        if self.count == 0 || self.count == u32::MAX {
            1
        } else {
            self.count.saturating_add(1)
        }
    }

    /// Collects up to `n` order ids from the front of the specified price level.
    pub fn get_order_ids(&self, price: u64, n: u32) -> Vec<u32> {
        let mut result = Vec::with_capacity(n as usize);
        let mut current = self.order_head.get(&price).copied().unwrap_or(0);

        while current != 0 {
            result.push(current);
            if result.len() as u32 >= n {
                break;
            }

            current = self
                .order_list
                .get(&price)
                .and_then(|level| level.get(&current))
                .copied()
                .unwrap_or(0);
        }

        result
    }

    /// Collects up to `n` orders from the front of the specified price level.
    pub fn get_orders(&self, price: u64, n: u32) -> Vec<Order> {
        self.get_order_ids(price, n)
            .into_iter()
            .filter_map(|id| self.orders.get(&id).cloned())
            .collect()
    }

    /// Collects orders within the `[start, end)` window from the specified price level.
    pub fn get_orders_in_range(&self, price: u64, start: u32, end: u32) -> Vec<Order> {
        if start >= end {
            return Vec::new();
        }

        let mut current_index = 0;
        let mut current = self.order_head.get(&price).copied().unwrap_or(0);
        let mut result = Vec::with_capacity((end - start) as usize);

        while current != 0 {
            if current_index >= start && current_index < end {
                if let Some(order) = self.orders.get(&current) {
                    result.push(order.clone());
                }
            }

            if current_index >= end {
                break;
            }

            current = self
                .order_list
                .get(&price)
                .and_then(|level| level.get(&current))
                .copied()
                .unwrap_or(0);
            current_index += 1;
        }

        result
    }

    pub fn head(&self, price: u64) -> Option<u32> {
        match self.order_head.get(&price).copied().unwrap_or(0) {
            0 => None,
            head => Some(head),
        }
    }

    pub fn tail(&self, price: u64) -> Option<u32> {
        match self.order_tail.get(&price).copied().unwrap_or(0) {
            0 => None,
            tail => Some(tail),
        }
    }

    pub fn is_empty(&self, price: u64) -> bool {
        self.order_head.get(&price).copied().unwrap_or(0) == 0
    }

    pub fn next(&self, price: u64, current: u32) -> Option<u32> {
        self.order_list
            .get(&price)
            .and_then(|lvl| lvl.get(&current))
            .copied()
            .filter(|next| *next != 0)
    }

    pub fn get_order(&self, id: u32) -> Option<&Order> {
        self.orders.get(&id)
    }
}
