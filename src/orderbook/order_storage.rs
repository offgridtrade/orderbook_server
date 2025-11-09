use std::collections::BTreeMap;

/// Represents an order stored in the order book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub owner: String,
    pub price: u128,
    pub deposit_amount: u128,
}

impl Order {
    pub fn new(owner: impl Into<String>, price: u128, deposit_amount: u128) -> Self {
        Self {
            owner: owner.into(),
            price,
            deposit_amount,
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
    list: BTreeMap<u128, BTreeMap<u32, u32>>,
    /// Order details keyed by id.
    orders: BTreeMap<u32, Order>,
    /// Mapping price -> head of the linked list.
    head: BTreeMap<u128, u32>,
    /// Sequential counter for new order ids.
    count: u32,
    /// Last displaced order when IDs collide.
    pub dormant_order: Option<Order>,
}

impl OrderStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_price(price: u128) -> Result<(), OrderbookError> {
        if price == 0 {
            Err(OrderbookError::PriceIsZero)
        } else {
            Ok(())
        }
    }

    /// Inserts an order id into the linked structure for a given price level,
    /// keeping the list sorted by `deposit_amount` (descending).
    pub fn insert_id(&mut self, price: u128, id: u32, amount: u128) -> Result<(), OrderbookError> {
        Self::ensure_price(price)?;

        let head_entry = self.head.entry(price).or_insert(0);
        let level_list = self.list.entry(price).or_default();

        let head_id = *head_entry;
        if head_id == 0
            || amount
                > self
                    .orders
                    .get(&head_id)
                    .map(|o| o.deposit_amount)
                    .unwrap_or(0)
        {
            level_list.insert(id, head_id);
            *head_entry = id;
            return Ok(());
        }

        let mut current = head_id;
        while current != 0 {
            let next = *level_list.get(&current).unwrap_or(&0);
            let next_amount = self
                .orders
                .get(&next)
                .map(|o| o.deposit_amount)
                .unwrap_or(0);

            if amount < next_amount {
                current = next;
                continue;
            } else if amount > next_amount {
                if next_amount == 0 {
                    level_list.insert(current, id);
                    level_list.insert(id, 0);
                    return Ok(());
                }

                level_list.insert(current, id);
                level_list.insert(id, next);
                return Ok(());
            } else {
                let after_next = next;
                let after_after = level_list.get(&after_next).copied().unwrap_or(0);
                level_list.insert(id, after_after);
                level_list.insert(after_next, id);
                return Ok(());
            }
        }

        Ok(())
    }

    /// Removes and returns the first order id at the given price level.
    pub fn pop_front(&mut self, price: u128) -> Option<u32> {
        let head_id = self.head.get_mut(&price)?;
        if *head_id == 0 {
            return None;
        }

        let first = *head_id;
        let next = self
            .list
            .get_mut(&price)
            .and_then(|lvl| lvl.remove(&first))
            .unwrap_or(0);

        *head_id = next;
        if next == 0 {
            self.list.remove(&price);
            self.head.remove(&price);
        }
        Some(first)
    }

    /// Creates a new order, assigning the next id. Returns `(id, found_dormant)`.
    pub fn create_order(
        &mut self,
        owner: impl Into<String>,
        price: u128,
        deposit_amount: u128,
    ) -> Result<(u32, bool), OrderbookError> {
        Self::ensure_price(price)?;

        let order = Order::new(owner, price, deposit_amount);
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
        amount: u128,
        dust: u128,
        clear: bool,
    ) -> Result<(u128, Option<u128>), OrderbookError> {
        if id == 0 {
            return Err(OrderbookError::OrderIdIsZero(id));
        }

        let (amount_to_send, should_delete) = match self.orders.get_mut(&id) {
            Some(order) => {
                let original = order.deposit_amount;
                let decreased = original.saturating_sub(amount);

                if decreased <= dust || clear {
                    (original, true)
                } else {
                    order.deposit_amount = decreased;
                    (amount.min(original), false)
                }
            }
            None => return Ok((0, None)),
        };

        if should_delete {
            let delete_price = self.delete_order(id);
            Ok((amount_to_send, delete_price))
        } else {
            Ok((amount_to_send, None))
        }
    }

    /// Deletes an order from the storage, returning the price level if it becomes empty.
    pub fn delete_order(&mut self, id: u32) -> Option<u128> {
        if id == 0 {
            return None;
        }

        let price = self.orders.get(&id)?.price;
        let head_id = *self.head.get(&price).unwrap_or(&0);

        if head_id == id {
            let next = self
                .list
                .get_mut(&price)
                .and_then(|lvl| lvl.remove(&head_id))
                .unwrap_or(0);
            if next == 0 {
                self.head.remove(&price);
                self.list.remove(&price);
            } else {
                self.head.insert(price, next);
            }
        } else {
            if let Some(level) = self.list.get_mut(&price) {
                let mut current = head_id;
                while current != 0 {
                    let next = level.get(&current).copied().unwrap_or(0);
                    if next == id {
                        let next_next = level.get(&next).copied().unwrap_or(0);
                        level.insert(current, next_next);
                        level.remove(&id);
                        break;
                    }
                    current = next;
                }
            }
        }

        self.orders.remove(&id);
        if self.head.get(&price).copied().unwrap_or(0) == 0 {
            self.head.remove(&price);
            self.list.remove(&price);
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
    pub fn get_order_ids(&self, price: u128, n: u32) -> Vec<u32> {
        let mut result = Vec::with_capacity(n as usize);
        let mut current = *self.head.get(&price).unwrap_or(&0);
        let level = self.list.get(&price);

        while current != 0 && (result.len() as u32) < n {
            result.push(current);
            current = level
                .and_then(|lvl| lvl.get(&current))
                .copied()
                .unwrap_or(0);
        }

        result
    }

    /// Collects up to `n` orders from the front of the specified price level.
    pub fn get_orders(&self, price: u128, n: u32) -> Vec<Order> {
        self.get_order_ids(price, n)
            .into_iter()
            .filter_map(|id| self.orders.get(&id).cloned())
            .collect()
    }

    /// Collects orders within the `[start, end)` window from the specified price level.
    pub fn get_orders_paginated(&self, price: u128, start: u32, end: u32) -> Vec<Order> {
        if start >= end {
            return Vec::new();
        }

        let mut current_index = 0;
        let mut current = *self.head.get(&price).unwrap_or(&0);
        let mut result = Vec::with_capacity((end - start) as usize);
        let level = self.list.get(&price);

        while current != 0 && current_index < start {
            current = level
                .and_then(|lvl| lvl.get(&current))
                .copied()
                .unwrap_or(0);
            current_index += 1;
        }

        while current != 0 && current_index < end {
            if let Some(order) = self.orders.get(&current) {
                result.push(order.clone());
            }
            current = level
                .and_then(|lvl| lvl.get(&current))
                .copied()
                .unwrap_or(0);
            current_index += 1;
        }

        result
    }

    pub fn head(&self, price: u128) -> Option<u32> {
        let head = self.head.get(&price).copied().unwrap_or(0);
        if head == 0 { None } else { Some(head) }
    }

    pub fn is_empty(&self, price: u128) -> bool {
        self.head.get(&price).copied().unwrap_or(0) == 0
    }

    pub fn next(&self, price: u128, current: u32) -> Option<u32> {
        self.list
            .get(&price)
            .and_then(|lvl| lvl.get(&current))
            .copied()
            .filter(|id| *id != 0)
    }

    pub fn get_order(&self, id: u32) -> Option<&Order> {
        self.orders.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_orders() -> OrderStorage {
        let mut storage = OrderStorage::new();
        let (id1, _) = storage.create_order("alice", 100, 50).unwrap();
        storage.insert_id(100, id1, 50).unwrap();

        let (id2, _) = storage.create_order("bob", 100, 75).unwrap();
        storage.insert_id(100, id2, 75).unwrap();

        let (id3, _) = storage.create_order("carol", 100, 20).unwrap();
        storage.insert_id(100, id3, 20).unwrap();

        storage
    }

    #[test]
    fn inserts_orders_by_deposit_desc() {
        let storage = setup_orders();
        let ids = storage.get_order_ids(100, 3);
        assert_eq!(ids.len(), 3);

        let first_order = storage.get_order(ids[0]).unwrap();
        assert_eq!(first_order.owner, "bob");
        let second_order = storage.get_order(ids[1]).unwrap();
        assert_eq!(second_order.owner, "alice");
        let third_order = storage.get_order(ids[2]).unwrap();
        assert_eq!(third_order.owner, "carol");
    }

    #[test]
    fn pop_front_removes_head() {
        let mut storage = setup_orders();
        let front = storage.pop_front(100);
        assert!(front.is_some());

        let ids = storage.get_order_ids(100, 3);
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn decrease_order_removes_when_below_dust() {
        let mut storage = OrderStorage::new();
        let (id, _) = storage.create_order("alice", 100, 75).unwrap();
        storage.insert_id(100, id, 75).unwrap();

        let (sent, deleted_price) = storage.decrease_order(id, 100, 1, false).unwrap();
        assert_eq!(sent, 75);
        assert_eq!(deleted_price, Some(100));
        assert!(storage.is_empty(100));
    }
}
