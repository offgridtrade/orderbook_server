use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use ulid::Ulid;

pub type OrderId = Ulid;

/// Represents an order stored in the order book.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Order {
    /// client order id
    pub cid: Vec<u8>,
    /// order id
    pub id: OrderId,
    /// owner of the order
    pub owner: Vec<u8>,
    /// is bid order
    pub is_bid: bool,
    /// price of the order in 8 decimals
    pub price: u64,
    /// whole amount of the order in 8 decimals without iceberg protection
    pub amnt: u64,
    /// iceberg quantity of the order in 8 decimals to hide the order from the public
    pub iqty: u64,
    /// public quantity of the order in 8 decimals with iceberg protection
    pub pqty: u64,
    /// current quantity of the order in 8 decimals without iceberg protection
    pub cqty: u64,
    /// timestamp of the order in milliseconds
    pub timestamp: i64,
    /// expires at timestamp in milliseconds
    pub expires_at: i64,
    /// maker fee basis points of the order
    pub maker_fee_bps: u16,
}

impl Order {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cid: Vec<u8>,
        id: OrderId,
        owner: Vec<u8>,
        is_bid: bool,
        price: u64,
        amnt: u64,
        iqty: u64,
        pqty: u64,
        cqty: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
    ) -> Self {
        Self {
            cid,
            id,
            owner,
            is_bid,
            price,
            amnt,
            iqty,
            pqty,
            cqty,
            timestamp,
            expires_at,
            maker_fee_bps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Node {
    pub prev: Option<OrderId>,
    pub next: Option<OrderId>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum L3Error {
    #[error("price is zero")]
    PriceIsZero,
    #[error("order does not exist: {0}")]
    OrderDoesNotExist(OrderId),
    #[error("iceberg quantity is bigger than whole amount")]
    IcebergQuantityIsBiggerThanWholeAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct L3 {
    /// Mapping price -> head of the linked list in a price level.
    pub price_head: BTreeMap<u64, OrderId>,
    /// Mapping price -> last of the linked list in a price level.
    pub price_tail: BTreeMap<u64, OrderId>,
    /// Order details keyed by id. Mapping order_id -> Order.
    pub order_nodes: HashMap<OrderId, Node>,
    /// Mapping order_id -> Order.
    pub orders: HashMap<OrderId, Order>,
    /// dust limit to determine if the order should be deleted
    pub dust: u64,
    /// Last displaced order when IDs collide.
    pub dormant_order: Option<OrderId>,
}

impl L3 {
    pub fn new() -> Self {
        Self {
            price_head: BTreeMap::new(),
            price_tail: BTreeMap::new(),
            order_nodes: HashMap::new(),
            orders: HashMap::new(),
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
    pub fn insert_id(&mut self, price: u64, id: OrderId, _amount: u128) -> Result<(), L3Error> {
        Self::ensure_price(price)?;
        // ensure the order exists from the orders map
        let order_node = self.order_nodes.get_mut(&id).ok_or(L3Error::OrderDoesNotExist(id))?;

        let price_tail = self.price_tail.get(&price).copied();

        // if the price level is not empty
        if let Some(price_tail) = price_tail {
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
    pub fn pop_front(&mut self, price: u64) -> Result<(Option<Order>, bool), L3Error> {
        Self::ensure_price(price)?;
        let head_id = self.price_head.get(&price).copied();
        if let Some(head_id) = head_id {
            let order_node = self
                .order_nodes
                .get_mut(&head_id)
                .ok_or(L3Error::OrderDoesNotExist(head_id))?;
            let next = order_node.next;
            if let Some(next) = next {
                order_node.next = None;
                let next_node = self
                    .order_nodes
                    .get_mut(&next)
                    .ok_or(L3Error::OrderDoesNotExist(next))?;
                next_node.prev = None;
                // set next node as the new head of the price level
                self.price_head.insert(price, next);
                // return order from the head_id value
                let order = self.orders.get(&head_id).ok_or(L3Error::OrderDoesNotExist(head_id))?;
                Ok((Some(order.clone()), false))
            } else {
                self.price_head.remove(&price);
                self.price_tail.remove(&price);
                // return order from the head_id value
                let order = self.orders.get(&head_id).ok_or(L3Error::OrderDoesNotExist(head_id))?;
                Ok((Some(order.clone()), true))
            }
        } else {
            Ok((None, true))
        }
    }

    pub fn set_iceberg_quantity(&mut self, id: OrderId, iqty: u64) -> Result<Order, L3Error> {
        let order = self.orders.get_mut(&id).ok_or(L3Error::OrderDoesNotExist(id))?;
        // update iqty of the order and public quantity of the order
        order.iqty = iqty;
        // update pqty from the difference between amnt and iqty
        if order.iqty > order.amnt {
            return Err(L3Error::IcebergQuantityIsBiggerThanWholeAmount);
        }

        let new_pqty = order.amnt - order.iqty;

        order.pqty = if order.cqty >= new_pqty { new_pqty } else { order.cqty };
        Ok(order.clone())
    }

    /// Creates a new order, assigning the next id. Returns the new order id.
    pub fn create_order(
        &mut self,
        cid: impl Into<Vec<u8>>,
        owner: impl Into<Vec<u8>>,
        is_bid: bool,
        price: u64,
        amnt: u64,
        iqty: u64,
        timestamp: i64,
        expires_at: i64,
        maker_fee_bps: u16,
    ) -> Result<Order, L3Error> {
        Self::ensure_price(price)?;
        let cid = cid.into();
        // generate a new order id
        let id = Ulid::new();
        let owner = owner.into();
        if iqty > amnt {
            return Err(L3Error::IcebergQuantityIsBiggerThanWholeAmount);
        }
        let pqty = amnt - iqty;
        let order = Order::new(
            cid,
            id,
            owner,
            is_bid,
            price,
            amnt,
            iqty,
            pqty,
            amnt,
            timestamp,
            expires_at,
            maker_fee_bps,
        );

        // Create a new node for the order
        self.order_nodes.insert(
            id,
            Node {
                prev: None,
                next: None,
            },
        );
        self.orders.insert(id, order.clone());
        self.insert_id(price, id, amnt as u128)?;

        Ok(order)
    }

    /// Decreases the deposit amount for a given order id.
    ///
    /// Returns `(amount_to_send, deleted_price_if_level_empty)` where:
    /// - `amount_to_send` is the liquidity that should be returned to the caller. based on the dust limit and the clear flag.
    /// - `deleted_price_if_level_empty` is `Some(price)` when the price level becomes empty.
    pub fn decrease_order(
        &mut self,
        id: OrderId,
        amount: u64,
        dust: u64,
        clear: bool,
    ) -> Result<(u64, Option<u64>), L3Error> {

        let mut amount_to_send;
        let mut should_delete = false;

        {
            let order = match self.orders.get_mut(&id) {
                Some(order) => order,
                None => return Ok((0, None)),
            };

            let original = order.cqty;
            amount_to_send = amount.min(original);
            let decreased = original.saturating_sub(amount_to_send);

            if clear || decreased <= dust {
                amount_to_send = original;
                should_delete = true;
            } else {
                // update the current base quantity of the order
                order.cqty = decreased;
                // update the current public quantity of the order
                // if pqty is bigger than cqty, keep pqty unchanged, otherwise set pqty to cqty as it does not need to hide anymore
                order.pqty = if order.cqty >= order.pqty { order.pqty } else { order.cqty };
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
    pub fn delete_order(&mut self, id: OrderId) -> Result<Option<u64>, L3Error> {

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
        
        let mut emptied_price = None;
        // connect prev and next nodes
        // if both prev and next are some, connect prev to next
        if let (Some(prev), Some(next)) = (prev, next) {
            let prev_node = self
                .order_nodes
                .get_mut(&prev)
                .ok_or(L3Error::OrderDoesNotExist(prev))?;
            prev_node.next = Some(next);
        }
        // if prev is some and next is none, make prev the tail of the price level
        else if let Some(prev) = prev {
            let prev_node = self
                .order_nodes
                .get_mut(&prev)
                .ok_or(L3Error::OrderDoesNotExist(prev))?;
            prev_node.next = None;
            self.price_tail.insert(price, prev);
        }
        // if prev is none and next is some, make next the head of the price level
        else if let Some(next) = next {
            let next_node = self
                .order_nodes
                .get_mut(&next)
                .ok_or(L3Error::OrderDoesNotExist(next))?;
            next_node.prev = None;
            self.price_head.insert(price, next);
        }
        // if both prev and next are none, the price level is empty, and return the price level
        else {
            self.price_head.remove(&price);
            self.price_tail.remove(&price);
            self.order_nodes.remove(&id);
            emptied_price = Some(price);
        }
        
        // remove order from the orders map
        self.orders.remove(&id);
        Ok(emptied_price)
    }

    /// Returns the next id that would be assigned on order creation.
    pub fn next_make_id(&self) -> OrderId {
        Ulid::new()
    }

    /// Collects up to `n` order ids from the front of the specified price level.
    pub fn get_order_ids(&self, price: u64, n: u32) -> Vec<OrderId> {
        let mut result = Vec::with_capacity(n as usize);
        let mut current = self.price_head.get(&price).copied();

        while let Some(id) = current {
            result.push(id);
            if result.len() as u32 >= n {
                break;
            }

            current = self.order_nodes.get(&id).and_then(|node| node.next);
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
        let mut current = self.price_head.get(&price).copied();
        let mut result = Vec::with_capacity((end - start) as usize);

        while let Some(id) = current {
            if current_index >= start && current_index < end {
                if let Some(order) = self.orders.get(&id) {
                    result.push(order.clone());
                }
            }

            if current_index >= end {
                break;
            }

            current = self.order_nodes.get(&id).and_then(|node| node.next);
            current_index += 1;
        }

        result
    }

    pub fn head(&self, price: u64) -> Option<OrderId> {
        self.price_head.get(&price).copied()
    }

    pub fn tail(&self, price: u64) -> Option<OrderId> {
        self.price_tail.get(&price).copied()
    }

    pub fn is_empty(&self, price: u64) -> bool {
        self.price_head.get(&price).is_none()
    }

    pub fn next(&self, _price: u64, current: OrderId) -> Option<OrderId> {
        // get the next node in the price level from the current node
        let next_node = self.order_nodes.get(&current).and_then(|node| node.next);
        match next_node {
            Some(next) => Some(next),
            None => None,   
        }
    }

    pub fn get_order(&self, id: OrderId) -> Result<&Order, L3Error> {
        self.orders.get(&id).ok_or(L3Error::OrderDoesNotExist(id))
    }

    /// Remove orders that have expired. Returns removed order ids.
    pub fn remove_dormant_orders(&mut self, now: i64) -> Vec<(OrderId, Order)> {
        let expired_orders: Vec<(OrderId, Order)> = self
            .orders
            .iter()
            .filter_map(|(id, order)| {
                if order.expires_at <= now {
                    Some((*id, order.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (id, _) in &expired_orders {
            let _ = self.delete_order(*id);
        }

        expired_orders
    }
}
