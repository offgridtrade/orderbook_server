use std::sync::Arc;

use thiserror::Error;

use super::DENOM;
use super::order_storage::{Order, OrderStorage, OrderbookError};
use super::price_linked_list::{PriceLinkedList, PriceListError};

pub trait MatchingEngine: Send + Sync {
    fn weth(&self) -> &str;
    fn fee_of(&self, base: &str, quote: &str, to: &str, is_maker: bool) -> u32;
    fn fee_to(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub id: u64,
    pub base: String,
    pub quote: String,
    pub engine: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferInstruction {
    pub token: String,
    pub to: String,
    pub amount: u128,
    pub fee_amount: u128,
    pub apply_fee: bool,
    pub is_maker: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderMatch {
    pub taker: String,
    pub maker: String,
    pub base_amount: u128,
    pub quote_amount: u128,
    pub base_fee: u128,
    pub quote_fee: u128,
    pub trade_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderPlacement {
    pub id: u32,
    pub found_dormant: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DormantRemoval {
    pub order: Order,
    pub transfers: Vec<TransferInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationResult {
    pub remaining: u128,
    pub transfers: Vec<TransferInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub order_match: OrderMatch,
    pub transfers: Vec<TransferInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FPopResult {
    pub order_id: u32,
    pub required: u128,
    pub clear: bool,
}

#[derive(Debug, Error)]
pub enum OrderbookInitError {
    #[error("invalid decimals: base={base}, quote={quote}")]
    InvalidDecimals { base: u8, quote: u8 },
}

#[derive(Debug, Error)]
pub enum OrderbookServiceError {
    #[error(transparent)]
    Orderbook(#[from] OrderbookError),
    #[error(transparent)]
    PriceList(#[from] PriceListError),
    #[error("price is zero")]
    PriceIsZero,
    #[error("order not found: {0}")]
    OrderNotFound(u32),
    #[error("invalid owner: expected={expected}, actual={actual}")]
    InvalidOwner { expected: String, actual: String },
    #[error("invalid decimals: base={base}, quote={quote}")]
    InvalidDecimals { base: u8, quote: u8 },
}

pub struct Orderbook {
    pair: Pair,
    dec_diff: u64,
    base_b_quote: bool,
    price_lists: PriceLinkedList,
    ask_orders: OrderStorage,
    bid_orders: OrderStorage,
    trade_count: u64,
    engine: Arc<dyn MatchingEngine>,
}

impl Orderbook {
    pub fn new(
        pair: Pair,
        base_decimals: u8,
        quote_decimals: u8,
        engine: Arc<dyn MatchingEngine>,
    ) -> Result<Self, OrderbookInitError> {
        if base_decimals > 18 || quote_decimals > 18 {
            return Err(OrderbookInitError::InvalidDecimals {
                base: base_decimals,
                quote: quote_decimals,
            });
        }

        let (diff, base_b_quote) = absdiff(base_decimals, quote_decimals);
        let dec_diff = 10u64.pow(diff as u32);

        Ok(Self {
            pair,
            dec_diff,
            base_b_quote,
            price_lists: PriceLinkedList::new(),
            ask_orders: OrderStorage::new(),
            bid_orders: OrderStorage::new(),
            trade_count: 0,
            engine,
        })
    }

    pub fn set_lmp(&mut self, price: u128) -> Result<(), OrderbookServiceError> {
        if price == 0 {
            return Err(OrderbookServiceError::PriceIsZero);
        }
        self.price_lists.set_lmp(price);
        Ok(())
    }

    pub fn place_ask(
        &mut self,
        owner: impl Into<String>,
        price: u128,
        amount: u128,
    ) -> Result<OrderPlacement, OrderbookServiceError> {
        self.clear_empty_head(false);

        let (id, found_dormant) = self.ask_orders.create_order(owner, price, amount)?;
        if self.ask_orders.is_empty(price) {
            self.price_lists.insert(false, price)?;
        }
        self.ask_orders.insert_id(price, id, amount)?;

        Ok(OrderPlacement { id, found_dormant })
    }

    pub fn place_bid(
        &mut self,
        owner: impl Into<String>,
        price: u128,
        amount: u128,
    ) -> Result<OrderPlacement, OrderbookServiceError> {
        self.clear_empty_head(true);

        let (id, found_dormant) = self.bid_orders.create_order(owner, price, amount)?;
        if self.bid_orders.is_empty(price) {
            self.price_lists.insert(true, price)?;
        }
        self.bid_orders.insert_id(price, id, amount)?;

        Ok(OrderPlacement { id, found_dormant })
    }

    pub fn remove_dormant(
        &mut self,
        is_bid: bool,
    ) -> Result<Option<DormantRemoval>, OrderbookServiceError> {
        let storage = if is_bid {
            &mut self.bid_orders
        } else {
            &mut self.ask_orders
        };

        let order = match storage.dormant_order.take() {
            Some(order) => order,
            None => return Ok(None),
        };

        let token = if is_bid {
            self.pair.quote.clone()
        } else {
            self.pair.base.clone()
        };

        let transfer = self.send_funds(
            token,
            order.owner.clone(),
            order.deposit_amount,
            false,
            false,
        );

        if self.is_empty(is_bid, order.price) {
            self.price_lists.delete(is_bid, order.price)?;
        }

        Ok(Some(DormantRemoval {
            order,
            transfers: vec![transfer],
        }))
    }

    pub fn cancel_order(
        &mut self,
        is_bid: bool,
        order_id: u32,
        owner: &str,
    ) -> Result<CancellationResult, OrderbookServiceError> {
        let order = {
            let storage_ref = if is_bid {
                &self.bid_orders
            } else {
                &self.ask_orders
            };
            storage_ref
                .get_order(order_id)
                .cloned()
                .ok_or(OrderbookServiceError::OrderNotFound(order_id))?
        };

        let storage = if is_bid {
            &mut self.bid_orders
        } else {
            &mut self.ask_orders
        };

        if order.owner != owner {
            return Err(OrderbookServiceError::InvalidOwner {
                expected: order.owner,
                actual: owner.to_string(),
            });
        }

        let was_empty = storage.is_empty(order.price);
        let delete_price = storage.delete_order(order_id);

        let token = if is_bid {
            self.pair.quote.clone()
        } else {
            self.pair.base.clone()
        };

        let transfer = self.send_funds(
            token,
            order.owner.clone(),
            order.deposit_amount,
            false,
            false,
        );

        if !was_empty {
            if let Some(price) = delete_price {
                self.price_lists.delete(is_bid, price)?;
            }
        }

        Ok(CancellationResult {
            remaining: order.deposit_amount,
            transfers: vec![transfer],
        })
    }

    pub fn execute(
        &mut self,
        order_id: u32,
        is_bid: bool,
        sender: &str,
        amount: u128,
        clear: bool,
    ) -> Result<ExecutionResult, OrderbookServiceError> {
        let order = if is_bid {
            self.bid_orders
                .get_order(order_id)
                .cloned()
                .ok_or(OrderbookServiceError::OrderNotFound(order_id))?
        } else {
            self.ask_orders
                .get_order(order_id)
                .cloned()
                .ok_or(OrderbookServiceError::OrderNotFound(order_id))?
        };

        let converted = self.convert(order.price, amount, is_bid);
        let dust = self.convert(order.price, 1, is_bid);

        let storage = if is_bid {
            &mut self.bid_orders
        } else {
            &mut self.ask_orders
        };

        let token_base = self.pair.base.clone();
        let token_quote = self.pair.quote.clone();

        if is_bid {
            let (with_dust, delete_price) =
                storage.decrease_order(order_id, converted, dust, clear)?;

            let base_transfer =
                self.send_funds(token_base.clone(), order.owner.clone(), amount, true, true);
            let quote_transfer = self.send_funds(
                token_quote.clone(),
                sender.to_string(),
                with_dust,
                true,
                false,
            );

            if let Some(price) = delete_price {
                self.price_lists.delete(true, price)?;
            }

            self.trade_count = self.next_trade_id();

            let order_match = OrderMatch {
                taker: sender.to_string(),
                maker: order.owner.clone(),
                base_amount: amount,
                quote_amount: converted,
                base_fee: base_transfer.fee_amount,
                quote_fee: quote_transfer.fee_amount,
                trade_id: self.trade_count,
            };

            Ok(ExecutionResult {
                order_match,
                transfers: vec![base_transfer, quote_transfer],
            })
        } else {
            let (with_dust, delete_price) =
                storage.decrease_order(order_id, converted, dust, clear)?;

            let quote_transfer =
                self.send_funds(token_quote.clone(), order.owner.clone(), amount, true, true);
            let base_transfer = self.send_funds(
                token_base.clone(),
                sender.to_string(),
                with_dust,
                true,
                false,
            );

            if let Some(price) = delete_price {
                self.price_lists.delete(false, price)?;
            }

            self.trade_count = self.next_trade_id();

            let order_match = OrderMatch {
                taker: sender.to_string(),
                maker: order.owner.clone(),
                base_amount: with_dust,
                quote_amount: amount,
                base_fee: base_transfer.fee_amount,
                quote_fee: quote_transfer.fee_amount,
                trade_id: self.trade_count,
            };

            Ok(ExecutionResult {
                order_match,
                transfers: vec![quote_transfer, base_transfer],
            })
        }
    }

    pub fn clear_empty_head(&mut self, is_bid: bool) -> u128 {
        let mut head = if is_bid {
            self.price_lists.bid_head()
        } else {
            self.price_lists.ask_head()
        };

        loop {
            if head == 0 {
                break;
            }

            let order_id = if is_bid {
                self.bid_orders.head(head)
            } else {
                self.ask_orders.head(head)
            };

            if order_id.is_some() {
                break;
            }

            head = self.price_lists.clear_head(is_bid);
        }

        head
    }

    pub fn fpop(
        &mut self,
        is_bid: bool,
        price: u128,
        remaining: u128,
    ) -> Result<FPopResult, OrderbookServiceError> {
        let order_id = if is_bid {
            self.bid_orders.head(price)
        } else {
            self.ask_orders.head(price)
        }
        .unwrap_or(0);
        if order_id == 0 {
            return Ok(FPopResult {
                order_id: 0,
                required: 0,
                clear: false,
            });
        }

        let order = if is_bid {
            self.bid_orders
                .get_order(order_id)
                .cloned()
                .ok_or(OrderbookServiceError::OrderNotFound(order_id))?
        } else {
            self.ask_orders
                .get_order(order_id)
                .cloned()
                .ok_or(OrderbookServiceError::OrderNotFound(order_id))?
        };

        let required = self.convert(price, order.deposit_amount, !is_bid);
        if required <= remaining {
            let storage = if is_bid {
                &mut self.bid_orders
            } else {
                &mut self.ask_orders
            };

            storage.pop_front(price);
            if storage.is_empty(price) {
                self.price_lists.delete(is_bid, price)?;
            }

            return Ok(FPopResult {
                order_id,
                required,
                clear: true,
            });
        }

        Ok(FPopResult {
            order_id,
            required,
            clear: false,
        })
    }

    pub fn get_required(
        &self,
        is_bid: bool,
        price: u128,
        order_id: u32,
    ) -> Result<u128, OrderbookServiceError> {
        let storage = if is_bid {
            &self.bid_orders
        } else {
            &self.ask_orders
        };

        let order = storage
            .get_order(order_id)
            .ok_or(OrderbookServiceError::OrderNotFound(order_id))?;

        if order.deposit_amount == 0 {
            return Ok(0);
        }

        Ok(self.convert(price, order.deposit_amount, is_bid))
    }

    pub fn lmp(&self) -> u128 {
        self.price_lists.lmp()
    }

    pub fn heads(&self) -> (u128, u128) {
        self.price_lists.heads()
    }

    pub fn ask_head(&self) -> u128 {
        self.price_lists.ask_head()
    }

    pub fn bid_head(&self) -> u128 {
        self.price_lists.bid_head()
    }

    pub fn order_head(&self, is_bid: bool, price: u128) -> Option<u32> {
        if is_bid {
            self.bid_orders.head(price)
        } else {
            self.ask_orders.head(price)
        }
    }

    pub fn mkt_price(&self) -> Result<u128, OrderbookServiceError> {
        Ok(self.price_lists.mkt_price()?)
    }

    pub fn get_prices(&self, is_bid: bool, n: usize) -> Vec<u128> {
        self.price_lists.get_prices(is_bid, n)
    }

    pub fn next_price(&self, is_bid: bool, price: u128) -> Option<u128> {
        self.price_lists.next(is_bid, price)
    }

    pub fn next_order(&self, is_bid: bool, price: u128, order_id: u32) -> Option<u32> {
        if is_bid {
            self.bid_orders.next(price, order_id)
        } else {
            self.ask_orders.next(price, order_id)
        }
    }

    pub fn sfpop(
        &self,
        is_bid: bool,
        price: u128,
        order_id: u32,
        is_head: bool,
    ) -> Result<FPopResult, OrderbookServiceError> {
        let id = if is_head {
            order_id
        } else {
            self.next_order(is_bid, price, order_id).unwrap_or(0)
        };

        if id == 0 {
            return Ok(FPopResult {
                order_id: 0,
                required: 0,
                clear: true,
            });
        }

        let storage = if is_bid {
            &self.bid_orders
        } else {
            &self.ask_orders
        };

        let order = storage
            .get_order(id)
            .ok_or(OrderbookServiceError::OrderNotFound(id))?;

        let required = self.convert(price, order.deposit_amount, !is_bid);
        Ok(FPopResult {
            order_id: id,
            required,
            clear: id == 0,
        })
    }

    pub fn get_prices_paginated(&self, is_bid: bool, start: usize, end: usize) -> Vec<u128> {
        self.price_lists.get_prices_paginated(is_bid, start, end)
    }

    pub fn get_order_ids(&self, is_bid: bool, price: u128, n: usize) -> Vec<u32> {
        if is_bid {
            self.bid_orders.get_order_ids(price, n as u32)
        } else {
            self.ask_orders.get_order_ids(price, n as u32)
        }
    }

    pub fn get_orders(&self, is_bid: bool, price: u128, n: usize) -> Vec<Order> {
        if is_bid {
            self.bid_orders.get_orders(price, n as u32)
        } else {
            self.ask_orders.get_orders(price, n as u32)
        }
    }

    pub fn get_orders_paginated(
        &self,
        is_bid: bool,
        price: u128,
        start: u32,
        end: u32,
    ) -> Vec<Order> {
        if is_bid {
            self.bid_orders.get_orders_paginated(price, start, end)
        } else {
            self.ask_orders.get_orders_paginated(price, start, end)
        }
    }

    pub fn get_order(&self, is_bid: bool, order_id: u32) -> Option<&Order> {
        if is_bid {
            self.bid_orders.get_order(order_id)
        } else {
            self.ask_orders.get_order(order_id)
        }
    }

    pub fn get_base_quote(&self) -> (&str, &str) {
        (&self.pair.base, &self.pair.quote)
    }

    pub fn asset_value(&self, amount: u128, is_bid: bool) -> Result<u128, OrderbookServiceError> {
        let price = self.price_lists.mkt_price()?;
        Ok(self.convert(price, amount, is_bid))
    }

    pub fn is_empty(&self, is_bid: bool, price: u128) -> bool {
        if is_bid {
            self.bid_orders.is_empty(price)
        } else {
            self.ask_orders.is_empty(price)
        }
    }

    pub fn convert_market(&self, amount: u128, is_bid: bool) -> u128 {
        self.convert(self.price_lists.lmp(), amount, is_bid)
    }

    pub fn convert(&self, price: u128, amount: u128, is_bid: bool) -> u128 {
        if price == 0 {
            return 0;
        }

        const E8: u128 = 100_000_000;
        let dec_diff = self.dec_diff as u128;

        if is_bid {
            if self.base_b_quote {
                ((amount * price) / E8) / dec_diff
            } else {
                ((amount * price) / E8) * dec_diff
            }
        } else if self.base_b_quote {
            ((amount * E8) / price) * dec_diff
        } else {
            ((amount * E8) / price) / dec_diff
        }
    }

    pub fn next_make_id(&self, is_bid: bool) -> u32 {
        if is_bid {
            self.bid_orders.next_make_id()
        } else {
            self.ask_orders.next_make_id()
        }
    }

    fn send_funds(
        &self,
        token: String,
        to: String,
        amount: u128,
        apply_fee: bool,
        is_maker: bool,
    ) -> TransferInstruction {
        let fee_amount = if apply_fee {
            let fee = self
                .engine
                .fee_of(&self.pair.base, &self.pair.quote, &to, is_maker);
            (amount * fee as u128) / DENOM as u128
        } else {
            0
        };

        TransferInstruction {
            token,
            to,
            amount,
            fee_amount,
            apply_fee,
            is_maker,
        }
    }

    fn next_trade_id(&self) -> u64 {
        if self.trade_count == 0 || self.trade_count == u64::MAX {
            1
        } else {
            self.trade_count + 1
        }
    }
}

fn absdiff(a: u8, b: u8) -> (u8, bool) {
    if a > b { (a - b, true) } else { (b - a, false) }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockEngine;

    impl MatchingEngine for MockEngine {
        fn weth(&self) -> &str {
            "weth"
        }

        fn fee_of(&self, _base: &str, _quote: &str, _to: &str, _is_maker: bool) -> u32 {
            0
        }

        fn fee_to(&self) -> &str {
            "fee"
        }
    }

    fn orderbook() -> Orderbook {
        let pair = Pair {
            id: 1,
            base: "BASE".into(),
            quote: "QUOTE".into(),
            engine: "engine".into(),
        };

        let engine: Arc<dyn MatchingEngine> = Arc::new(MockEngine);
        Orderbook::new(pair, 8, 8, engine).expect("init")
    }

    #[test]
    fn place_bid_and_ask_updates_prices() {
        let mut ob = orderbook();

        let bid = ob
            .place_bid("maker", 100_000_000, 500_000_000)
            .expect("place bid");
        assert_eq!(bid.id, 1);
        assert_eq!(ob.bid_head(), 100_000_000);
        assert_eq!(
            ob.get_order(true, bid.id).unwrap().deposit_amount,
            500_000_000
        );

        let ask = ob
            .place_ask("taker", 110_000_000, 400_000_000)
            .expect("place ask");
        assert_eq!(ask.id, 1);
        assert_eq!(ob.ask_head(), 110_000_000);

        assert_eq!(ob.get_prices(true, 1), vec![100_000_000]);
        assert_eq!(ob.get_prices(false, 1), vec![110_000_000]);
    }

    #[test]
    fn execute_bid_reduces_deposit_and_emits_match() {
        let mut ob = orderbook();
        let bid = ob
            .place_bid("maker", 100_000_000, 500_000_000)
            .expect("bid placed");

        let result = ob
            .execute(bid.id, true, "taker", 200_000_000, false)
            .expect("execute");

        assert_eq!(result.order_match.trade_id, 1);
        assert_eq!(result.order_match.base_amount, 200_000_000);
        assert_eq!(result.order_match.quote_amount, 200_000_000);
        assert_eq!(result.order_match.base_fee, 0);
        assert_eq!(result.order_match.quote_fee, 0);

        let remaining = ob.get_order(true, bid.id).unwrap().deposit_amount;
        assert_eq!(remaining, 300_000_000);
    }
}
