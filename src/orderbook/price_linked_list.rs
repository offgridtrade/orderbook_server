use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PriceListError {
    #[error(
        "no match price: bid_head={bid_head}, ask_head={ask_head}, last_match_price={last_match_price}"
    )]
    NoMatchPrice {
        bid_head: u128,
        ask_head: u128,
        last_match_price: u128,
    },
    #[error("price must be non-zero")]
    ZeroPrice,
    #[error("no head below: is_bid={is_bid}, head={head}")]
    NoHeadBelow { is_bid: bool, head: u128 },
    #[error("price out of range: reference={reference}, price={price}")]
    PriceOutOfRange { reference: u128, price: u128 },
    #[error("price none in range: reference={reference}, price={price}")]
    PriceNoneInRange { reference: u128, price: u128 },
}

#[derive(Debug, Default)]
pub struct PriceLinkedList {
    ask_prices: BTreeMap<u128, u128>,
    bid_prices: BTreeMap<u128, u128>,
    ask_head: u128,
    bid_head: u128,
    last_match_price: u128,
}

impl PriceLinkedList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_lmp(&mut self, lmp: u128) {
        self.last_match_price = lmp;
    }

    pub fn heads(&self) -> (u128, u128) {
        (self.bid_head, self.ask_head)
    }

    pub fn ask_head(&self) -> u128 {
        self.ask_head
    }

    pub fn bid_head(&self) -> u128 {
        self.bid_head
    }

    pub fn lmp(&self) -> u128 {
        self.last_match_price
    }

    pub fn mkt_price(&self) -> Result<u128, PriceListError> {
        match (self.bid_head, self.ask_head, self.last_match_price) {
            (0, 0, 0) => Err(PriceListError::NoMatchPrice {
                bid_head: 0,
                ask_head: 0,
                last_match_price: 0,
            }),
            (0, 0, lmp) => Ok(lmp),
            (bid, 0, lmp) if lmp != 0 => Ok(lmp.max(bid)),
            (bid, 0, _) => Ok(bid),
            (0, ask, lmp) if lmp != 0 => Ok(lmp.min(ask)),
            (0, ask, _) => Ok(ask),
            (_, _, lmp) => Ok(lmp),
        }
    }

    pub fn next(&self, is_bid: bool, price: u128) -> Option<u128> {
        if is_bid {
            self.bid_prices
                .get(&price)
                .copied()
                .filter(|next| *next != 0)
        } else {
            self.ask_prices
                .get(&price)
                .copied()
                .filter(|next| *next != 0)
        }
    }

    pub fn insert(&mut self, is_bid: bool, price: u128) -> Result<(), PriceListError> {
        if price == 0 {
            return Err(PriceListError::ZeroPrice);
        }

        if is_bid {
            self.insert_bid(price);
        } else {
            self.insert_ask(price);
        }
        Ok(())
    }

    fn insert_bid(&mut self, price: u128) {
        if self.bid_head == 0 || price > self.bid_head {
            self.bid_prices.insert(price, self.bid_head);
            self.bid_head = price;
            return;
        }

        let mut current = self.bid_head;
        loop {
            let next = self.bid_prices.get(&current).copied().unwrap_or(0);
            if price < next && next != 0 {
                current = next;
                continue;
            } else if price > next {
                if next == 0 {
                    self.bid_prices.insert(current, price);
                    self.bid_prices.insert(price, 0);
                    return;
                }
                self.bid_prices.insert(current, price);
                self.bid_prices.insert(price, next);
                return;
            } else if price == next {
                return;
            } else {
                // price < next but next == 0
                self.bid_prices.insert(current, price);
                self.bid_prices.insert(price, 0);
                return;
            }
        }
    }

    fn insert_ask(&mut self, price: u128) {
        if self.ask_head == 0 || price < self.ask_head {
            self.ask_prices.insert(price, self.ask_head);
            self.ask_head = price;
            return;
        }

        let mut current = self.ask_head;
        loop {
            let next = self.ask_prices.get(&current).copied().unwrap_or(0);
            if next == 0 {
                if price == current {
                    return;
                }
                self.ask_prices.insert(current, price);
                self.ask_prices.insert(price, 0);
                return;
            }

            if price > next {
                current = next;
                continue;
            } else if price < next {
                self.ask_prices.insert(current, price);
                self.ask_prices.insert(price, next);
                return;
            } else {
                return;
            }
        }
    }

    pub fn clear_head(&mut self, is_bid: bool) -> u128 {
        if is_bid {
            if self.bid_head != 0 {
                let next = self.bid_prices.get(&self.bid_head).copied().unwrap_or(0);
                self.bid_head = next;
            }
            self.bid_head
        } else {
            if self.ask_head != 0 {
                let next = self.ask_prices.get(&self.ask_head).copied().unwrap_or(0);
                self.ask_head = next;
            }
            self.ask_head
        }
    }

    pub fn delete(&mut self, is_bid: bool, price: u128) -> Result<bool, PriceListError> {
        if price == 0 {
            return Err(PriceListError::ZeroPrice);
        }

        if is_bid {
            self.delete_from_list(price, true)
        } else {
            self.delete_from_list(price, false)
        }
    }

    fn delete_from_list(&mut self, price: u128, is_bid: bool) -> Result<bool, PriceListError> {
        let (head, map) = if is_bid {
            (&mut self.bid_head, &mut self.bid_prices)
        } else {
            (&mut self.ask_head, &mut self.ask_prices)
        };

        if *head == 0 {
            return Err(PriceListError::NoHeadBelow {
                is_bid,
                head: *head,
            });
        }

        if (is_bid && price > *head) || (!is_bid && price < *head) {
            return Err(PriceListError::NoHeadBelow {
                is_bid,
                head: *head,
            });
        }

        let mut prev = None;
        let mut current = *head;
        while current != 0 {
            if current == price {
                let next = map.get(&current).copied().unwrap_or(0);
                match prev {
                    Some(prev_price) => {
                        map.insert(prev_price, next);
                    }
                    None => {
                        *head = next;
                    }
                }
                map.remove(&current);
                return Ok(true);
            }

            let next = map.get(&current).copied().unwrap_or(0);
            if next == 0 {
                return Err(PriceListError::PriceOutOfRange {
                    reference: current,
                    price,
                });
            }

            if (is_bid && price > next) || (!is_bid && price < next) {
                prev = Some(current);
                current = next;
                continue;
            }

            if next == price {
                let after_next = map.get(&next).copied().unwrap_or(0);
                map.insert(current, after_next);
                map.remove(&next);
                return Ok(true);
            }

            return Err(PriceListError::PriceNoneInRange {
                reference: current,
                price,
            });
        }

        Ok(false)
    }

    pub fn get_prices(&self, is_bid: bool, n: usize) -> Vec<u128> {
        let mut prices = Vec::with_capacity(n);
        let mut current = if is_bid { self.bid_head } else { self.ask_head };
        let map = if is_bid {
            &self.bid_prices
        } else {
            &self.ask_prices
        };

        while current != 0 && prices.len() < n {
            prices.push(current);
            current = map.get(&current).copied().unwrap_or(0);
        }

        prices
    }

    pub fn get_prices_paginated(&self, is_bid: bool, start: usize, end: usize) -> Vec<u128> {
        if start >= end {
            return Vec::new();
        }

        let map = if is_bid {
            &self.bid_prices
        } else {
            &self.ask_prices
        };
        let mut current = if is_bid { self.bid_head } else { self.ask_head };
        let mut index = 0;

        while current != 0 && index < start {
            current = map.get(&current).copied().unwrap_or(0);
            index += 1;
        }

        if current == 0 {
            return vec![0; end - start];
        }

        let mut result = Vec::with_capacity(end - start);
        while current != 0 && index < end {
            result.push(current);
            current = map.get(&current).copied().unwrap_or(0);
            index += 1;
        }

        while result.len() < end - start {
            result.push(0);
        }

        result
    }

    pub fn check_price_exists(&self, is_bid: bool, price: u128) -> Result<bool, PriceListError> {
        if price == 0 {
            return Err(PriceListError::ZeroPrice);
        }

        let (head, map) = if is_bid {
            (self.bid_head, &self.bid_prices)
        } else {
            (self.ask_head, &self.ask_prices)
        };

        if head == 0 || (is_bid && price > head) || (!is_bid && price < head) {
            return Err(PriceListError::NoHeadBelow { is_bid, head });
        }

        let mut current = head;
        while current != 0 {
            if current == price {
                return Ok(true);
            }

            let next = map.get(&current).copied().unwrap_or(0);
            if next == 0 {
                return Err(PriceListError::PriceOutOfRange {
                    reference: current,
                    price,
                });
            }

            if (is_bid && price > next) || (!is_bid && price < next) {
                current = next;
                continue;
            }

            if next == price {
                return Ok(true);
            }

            return Err(PriceListError::PriceNoneInRange {
                reference: current,
                price,
            });
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_bid_prices() {
        let mut list = PriceLinkedList::new();
        list.insert(true, 200).unwrap();
        list.insert(true, 150).unwrap();
        list.insert(true, 250).unwrap();

        assert_eq!(list.bid_head(), 250);
        assert_eq!(list.get_prices(true, 3), vec![250, 200, 150]);
    }

    #[test]
    fn insert_and_get_ask_prices() {
        let mut list = PriceLinkedList::new();
        list.insert(false, 200).unwrap();
        list.insert(false, 150).unwrap();
        list.insert(false, 250).unwrap();

        assert_eq!(list.ask_head(), 150);
        assert_eq!(list.get_prices(false, 3), vec![150, 200, 250]);
    }

    #[test]
    fn delete_bid_price() {
        let mut list = PriceLinkedList::new();
        list.insert(true, 300).unwrap();
        list.insert(true, 200).unwrap();
        list.insert(true, 100).unwrap();

        let removed = list.delete(true, 200).unwrap();
        assert!(removed);
        assert_eq!(list.get_prices(true, 3), vec![300, 100]);
    }

    #[test]
    fn delete_ask_price() {
        let mut list = PriceLinkedList::new();
        list.insert(false, 300).unwrap();
        list.insert(false, 200).unwrap();
        list.insert(false, 100).unwrap();

        let removed = list.delete(false, 200).unwrap();
        assert!(removed);
        assert_eq!(list.get_prices(false, 3), vec![100, 300]);
    }

    #[test]
    fn market_price_uses_heads() {
        let mut list = PriceLinkedList::new();
        list.insert(true, 200).unwrap();
        list.insert(false, 250).unwrap();
        list.set_lmp(220);

        assert_eq!(list.mkt_price().unwrap(), 220);
    }

    #[test]
    fn check_price_exists() {
        let mut list = PriceLinkedList::new();
        list.insert(true, 220).unwrap();
        list.insert(true, 210).unwrap();
        list.insert(true, 200).unwrap();

        assert!(list.check_price_exists(true, 210).unwrap());
    }
}
