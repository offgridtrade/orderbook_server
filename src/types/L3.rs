use std::collections::BTreeMap;

/// Represents an order stored in the order book.
#[derive(Clone, Debug, PartialEq, Eq)]
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
pub enum OrderbookError {
    #[error("order id is zero: {0}")]
    OrderIdIsZero(u32),
    #[error("price is zero")]
    PriceIsZero,
}

#[derive(Debug, Default)]
pub struct OrderStorage {
    /// Mapping price -> linked list stored as `current -> next`.
    order_list: BTreeMap<u64, BTreeMap<u32, u32>>,
    /// Mapping price -> head of the linked list.
    order_head: BTreeMap<u64, u32>,
    /// Mapping price -> last of the linked list.
    order_last: BTreeMap<u64, u32>,
    /// Order details keyed by id. Mapping order_id -> Order.
    orders: BTreeMap<u32, Order>,
    /// Sequential counter for new order ids.
    count: u32,
    /// Last displaced order when IDs collide.
    pub dormant_order: Option<Order>,
}

impl OrderStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_price(price: u64) -> Result<(), OrderbookError> {
        if price == 0 {
            Err(OrderbookError::PriceIsZero)
        } else {
            Ok(())
        }
    }

    /// Inserts an order id into the linked structure for a given price level,
    /// keeping FIFO (append at tail).
    pub fn insert_id(
        &mut self,
        price: u64,
        id: u32,
        _amount: u128,
    ) -> Result<(), OrderbookError> {
        Self::ensure_price(price)?;

        let level = self
            .order_list
            .entry(price)
            .or_insert_with(BTreeMap::new);

        match self.order_last.get(&price).and_then(|opt| *opt) {
            Some(last_id) => {
                level.insert(last_id, Some(id));
            }
            None => {
                self.order_head.insert(price, Some(id));
            }
        }

        level.insert(id, None);
        self.order_last.insert(price, Some(id));

        Ok(())
    }

    /// Removes and returns the first order id at the given price level.
    pub fn pop_front(&mut self, price: u64) -> Option<u32> {
        Self::ensure_price(price).ok()?;
        let head_id = self.order_head.get(&price).and_then(|opt| *opt)?;
        let next = self
            .order_list
            .get_mut(&price)
            .and_then(|level| level.remove(&head_id))
            .flatten();

        match next {
            Some(next_id) => {
                self.order_head.insert(price, Some(next_id));
            }
            None => {
                self.order_head.remove(&price);
                self.order_last.remove(&price);
                self.order_list.remove(&price);
            }
        }

        self.orders.remove(&head_id);
        Some(head_id)
    }

    /// Creates a new order, assigning the next id. Returns `(id, found_dormant)`.
    pub fn create_order(
        &mut self,
        cid: Vec<u8>,
        owner: Vec<u8>,
        price: u64,
        iq: u64,
        pq: u64,
        timestamp: i64,
    ) -> Result<(u32, bool), OrderbookError> {
        Self::ensure_price(price)?;
        let order = Order::new(cid, owner, price, pq, iq, iq, timestamp);

        self.count = if self.count == 0 || self.count == u32::MAX {
            1
        } else {
            self.count.saturating_add(1)
        };

        if self.orders.contains_key(&self.count) {
            self.dormant_order = self.orders.get(&self.count).cloned();
            self.delete_order(self.count);
            let found = self.dormant_order.is_some();
            self.orders.insert(self.count, order);
            return Ok((self.count, found));
        }

        self.orders.insert(self.count, order);
        Ok((self.count, false))
    }

    /// Decreases the deposit amount for a given order id.
    /// Returns `(amount_to_send, deleted_price_if_level_empty)`.
    pub fn decrease_order(
        &mut self,
        id: u32,
        amount: u64,
        dust: u64,
        clear: bool,
    ) -> Result<(u64, Option<u64>), OrderbookError> {
        if id == 0 {
            return Err(OrderbookError::OrderIdIsZero(id));
        }

        let mut amount_to_send = 0;
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
            let delete_price = self.delete_order(id);
            Ok((amount_to_send, delete_price))
        } else {
            Ok((amount_to_send, None))
        }
    }

    /// Deletes an order from the storage, returning the price level if it becomes empty.
    pub fn delete_order(&mut self, id: u32) -> Option<u64> {
        if id == 0 {
            return None;
        }

        let price = self.orders.get(&id)?.price;
        let head_id = self.order_head.get(&price).and_then(|opt| *opt);

        if head_id == Some(id) {
            let next = self
                .order_list
                .get_mut(&price)
                .and_then(|lvl| lvl.remove(&id))
                .flatten();
            match next {
                Some(next_id) => {
                    self.order_head.insert(price, Some(next_id));
                }
                None => {
                    self.order_head.remove(&price);
                    self.order_last.remove(&price);
                    self.order_list.remove(&price);
                }
            }
        } else if let Some(level) = self.order_list.get_mut(&price) {
            let mut current = head_id.flatten().unwrap_or(0);
            while current != 0 {
                let next = level.get(&current).copied().flatten();
                if next == Some(id) {
                    let next_next = level.remove(&id).flatten();
                    level.insert(current, next_next);
                    if next_next.is_none() {
                        self.order_last.insert(price, Some(current));
                    }
                    break;
                }
                current = next.unwrap_or(0);
            }
        }

        self.orders.remove(&id);

        if self.order_head.get(&price).and_then(|opt| *opt).is_none() {
            self.order_head.remove(&price);
            self.order_last.remove(&price);
            self.order_list.remove(&price);
            Some(price)
        } else {
            None
        }
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
        let mut current = self.order_head.get(&price).and_then(|opt| *opt);

        while let Some(current_id) = current {
            result.push(current_id);
            if result.len() as u32 >= n {
                break;
            }

            current = self
                .order_list
                .get(&price)
                .and_then(|level| level.get(&current_id))
                .copied()
                .flatten();
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
    pub fn get_orders_paginated(&self, price: u64, start: u32, end: u32) -> Vec<Order> {
        if start >= end {
            return Vec::new();
        }

        let mut current_index = 0;
        let mut current = self.order_head.get(&price).and_then(|opt| *opt);
        let mut result = Vec::with_capacity((end - start) as usize);

        while let Some(current_id) = current {
            if current_index >= start && current_index < end {
                if let Some(order) = self.orders.get(&current_id) {
                    result.push(order.clone());
                }
            }

            if current_index >= end {
                break;
            }

            current = self
                .order_list
                .get(&price)
                .and_then(|level| level.get(&current_id))
                .copied()
                .flatten();
            current_index += 1;
        }

        result
    }

    pub fn head(&self, price: u64) -> Option<u32> {
        self.order_head.get(&price).and_then(|opt| *opt)
    }

    pub fn is_empty(&self, price: u64) -> bool {
        self.head(price).is_none()
    }

    pub fn next(&self, price: u64, current: u32) -> Option<u32> {
        self.order_list
            .get(&price)
            .and_then(|lvl| lvl.get(&current))
            .copied()
            .flatten()
    }

    pub fn get_order(&self, id: u32) -> Option<&Order> {
        self.orders.get(&id)
    }
}
