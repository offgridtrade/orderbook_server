use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Node {
    pub prev: Option<u32>,
    pub next: Option<u32>,
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
    /// Mapping price -> head of the linked list in a price level.
    pub price_head: BTreeMap<u64, u32>,
    /// Mapping price -> last of the linked list in a price level.
    pub price_tail: BTreeMap<u64, u32>,
    /// Order details keyed by id. Mapping order_id -> Order.
    pub order_nodes: HashMap<u32, Node>,
    /// Mapping order_id -> Order.
    pub orders: HashMap<u32, Order>,
    /// Sequential counter for new order ids.
    pub count: u32,
    /// dust limit to determine if the order should be deleted
    pub dust: u64,
    /// Last displaced order when IDs collide.
    pub dormant_order: Option<Node>,
}

impl L3 {
    pub fn new() -> Self {
        Self {
            price_head: BTreeMap::new(),
            price_tail: BTreeMap::new(),
            order_nodes: HashMap::new(),
            orders: HashMap::new(),
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
        // ensure the order exists from the orders map
        let order_node = self.order_nodes.get_mut(&id).ok_or(L3Error::OrderDoesNotExist(id))?;

        let price_tail = self.price_tail.get(&price).copied().unwrap_or(0);

        // if the price level is not empty
        if price_tail != 0 {
            // insert the order node at the tail of the price level
            // First, update the prev of the new order node to point to old tail
            order_node.prev = Some(price_tail);
            order_node.next = None;
            // To avoid double mutable borrow of self.order_nodes, we do this after:
            let price_tail_node_ptr = self.order_nodes.get_mut(&price_tail).ok_or(L3Error::OrderDoesNotExist(price_tail))?;
            price_tail_node_ptr.next = Some(id);
            // Update the tail pointer to the new node
            self.price_tail.insert(price, id);
        } 
        // if the price level is empty
        else {
            // insert the order node at the head of the price level
            order_node.prev = None;
            order_node.next = None;
            self.price_head.insert(price, id);
            self.price_tail.insert(price, id);
        }

        Ok(())
    }

    /// Removes and returns the first order id at the given price level.
    /// returns (order_id, is_empty)
    /// - `order_id` is the id of the first order in the price level.
    /// - `is_empty` is true when the price level becomes empty.
    pub fn pop_front(&mut self, price: u64) -> Result<(Option<u32>, bool), L3Error> {
        Self::ensure_price(price)?;
        let head_id = self.price_head.get(&price).copied().unwrap_or(0);
        if head_id == 0 {
            return Ok((None, true));
        }
        let order_node = self.order_nodes.get_mut(&head_id).ok_or(L3Error::OrderDoesNotExist(head_id))?;
        let next = order_node.next;
        if next.is_some() {
            order_node.next = None;
            let next_node = self.order_nodes.get_mut(&next.unwrap()).ok_or(L3Error::OrderDoesNotExist(next.unwrap()))?;
            next_node.prev = None;
            // set next node as the new head of the price level
            self.price_head.insert(price, next.unwrap());
            Ok((Some(head_id), false))
        } else {
            self.price_head.remove(&price);
            self.price_tail.remove(&price);
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

        if let Some(existing) = self.order_nodes.get(&self.count).cloned() {
            self.dormant_order = Some(existing);
            self.delete_order(self.count)?;
            // update the prev and next of the existing node to None
            let existing_node = self.order_nodes.get_mut(&self.count).ok_or(L3Error::OrderDoesNotExist(self.count))?;
            existing_node.prev = None;
            existing_node.next = None;
            self.orders.insert(self.count, order);
            return Ok((self.count, true));
        }

        // Create a new node for the order
        self.order_nodes.insert(self.count, Node {
            prev: None,
            next: None,
        });
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

        // remove order node from the order nodes map
        let order_node = self.order_nodes.get_mut(&id).ok_or(L3Error::OrderDoesNotExist(id))?;
        let prev = order_node.prev;
        let next = order_node.next;
        
        // connect prev and next nodes
        // if both prev and next are some, connect prev to next
        if prev.is_some() && next.is_some() {
            let prev_node = self.order_nodes.get_mut(&prev.unwrap()).ok_or(L3Error::OrderDoesNotExist(prev.unwrap()))?;
            prev_node.next = next;
        }
        // if prev is some and next is none, make prev the tail of the price level
        else if prev.is_some()  && next.is_none(){
            let prev_node = self.order_nodes.get_mut(&prev.unwrap()).ok_or(L3Error::OrderDoesNotExist(prev.unwrap()))?;
            prev_node.next = None;
            self.price_tail.insert(price, prev.unwrap());
           
        }
        // if prev is none and next is some, make next the head of the price level
        else if prev.is_none() && next.is_some() {
            let next_node = self.order_nodes.get_mut(&next.unwrap()).ok_or(L3Error::OrderDoesNotExist(next.unwrap()))?;
            next_node.prev = None;
            self.price_head.insert(price, next.unwrap());
        }
        // if both prev and next are none, the price level is empty, and return the price level
        else {
            self.price_head.remove(&price);
            self.price_tail.remove(&price);
            self.order_nodes.remove(&id);
            return Ok(Some(price));
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
        let mut current = self.price_head.get(&price).copied().unwrap_or(0);

        while current != 0 {
            result.push(current);
            if result.len() as u32 >= n {
                break;
            }

            current = self
                .order_nodes
                .get(&current)
                .and_then(|node| node.next)
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
        let mut current = self.price_head.get(&price).copied().unwrap_or(0);
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
                .order_nodes
                .get(&current)
                .and_then(|node| node.next)
                .unwrap_or(0);
            current_index += 1;
        }

        result
    }

    pub fn head(&self, price: u64) -> Option<u32> {
        match self.price_head.get(&price).copied().unwrap_or(0) {
            0 => None,
            head => Some(head),
        }
    }

    pub fn tail(&self, price: u64) -> Option<u32> {
        match self.price_tail.get(&price).copied().unwrap_or(0) {
            0 => None,
            tail => Some(tail),
        }
    }

    pub fn is_empty(&self, price: u64) -> bool {
        self.price_head.get(&price).copied().unwrap_or(0) == 0
    }

    pub fn next(&self, _price: u64, current: u32) -> Option<u32> {
        // get the next node in the price level from the current node
        let next_node = self.order_nodes.get(&current).and_then(|node| node.next);
        match next_node {
            Some(next) => Some(next),
            None => None,   
        }
    }

    pub fn get_order(&self, id: u32) -> Result<&Order, L3Error> {
        self.orders.get(&id).ok_or(L3Error::OrderDoesNotExist(id))
    }
}
