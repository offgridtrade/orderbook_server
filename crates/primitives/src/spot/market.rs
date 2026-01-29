use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L1 {
    /// Last match price
    pub lmp: Option<u64>,
    /// Head of the bid list
    pub bid_head: Option<u64>,
    /// Head of the ask list
    pub ask_head: Option<u64>,
    /// Slippage limit for limit buy orders in 8 decimals
    pub limit_buy_slippage_limit: Option<u64>,
    /// Slippage limit for limit sell orders in 8 decimals
    pub limit_sell_slippage_limit: Option<u64>,
    /// Slippage limit for market buy orders in 8 decimals
    pub market_buy_slippage_limit: Option<u64>,
    /// Slippage limit for market sell orders in 8 decimals
    pub market_sell_slippage_limit: Option<u64>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum L1Error {
    #[error("price is zero")]
    PriceIsZero,
}

impl Default for L1 {
    fn default() -> Self {
        Self {
            lmp: None,
            bid_head: None,
            ask_head: None,
            limit_buy_slippage_limit: Some(10000u64),
            limit_sell_slippage_limit: Some(10000u64),
            market_buy_slippage_limit: Some(10000u64),
            market_sell_slippage_limit: Some(10000u64),
        }
    }
}

impl L1 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_slippage(
        lmp: Option<u64>,
        bid_head: Option<u64>,
        ask_head: Option<u64>,
        limit_buy_slippage_limit: Option<u64>,
        limit_sell_slippage_limit: Option<u64>,
        market_buy_slippage_limit: Option<u64>,
        market_sell_slippage_limit: Option<u64>,
    ) -> Self {
        Self {
            lmp,
            bid_head,
            ask_head,
            limit_buy_slippage_limit,
            limit_sell_slippage_limit,
            market_buy_slippage_limit,
            market_sell_slippage_limit,
        }
    }

    // ask head price
    pub fn ask_head(&self) -> Option<u64> {
        self.ask_head
    }

    // bid head price
    pub fn bid_head(&self) -> Option<u64> {
        self.bid_head
    }

    pub fn lmp(&self) -> Option<u64> {
        self.lmp
    }

    pub fn set_lmp(&mut self, price: u64) {
        self.lmp = Some(price);
    }

    pub fn set_ask_head(&mut self, price: u64) {
        self.ask_head = Some(price);
    }

    pub fn set_bid_head(&mut self, price: u64) {
        self.bid_head = Some(price);
    }

    /// Set the slippage limit for limit buy orders
    pub fn set_limit_buy_slippage_limit(&mut self, slippage_limit: Option<u64>) {
        self.limit_buy_slippage_limit = slippage_limit;
    }

    /// Set the slippage limit for limit sell orders
    pub fn set_limit_sell_slippage_limit(&mut self, slippage_limit: Option<u64>) {
        self.limit_sell_slippage_limit = slippage_limit;
    }

    /// Set the slippage limit for market buy orders
    pub fn set_market_buy_slippage_limit(&mut self, slippage_limit: Option<u64>) {
        self.market_buy_slippage_limit = slippage_limit;
    }

    /// Set the slippage limit for market sell orders
    pub fn set_market_sell_slippage_limit(&mut self, slippage_limit: Option<u64>) {
        self.market_sell_slippage_limit = slippage_limit;
    }

    /// Determine the maker price for a limit sell order
    /// This function calculates the price at which a limit sell order should be placed
    /// as a maker order based on current market conditions, limit price, and spread.
    /// 
    /// - `lp`: Limit price specified by the user
    /// - `bid_head`: Current best bid price (0 if no bids)
    /// - `ask_head`: Current best ask price (0 if no asks)
    /// - `spread`: Spread in basis points (10000 = 100%)
    /// - Returns the calculated maker price and lmp tuple
    #[cfg_attr(test, allow(dead_code))]
    pub fn det_limit_sell_make_price(
        &self,
        lp: u64,
        bid_head: u64,
        ask_head: u64,
        spread: u32,
    ) -> (u64, u64) {
        const DENOM: u64 = 10000; // Basis points denominator (10000 = 100%)
        let lmp = self.lmp().unwrap_or(0);

        if ask_head == 0 && bid_head == 0 {
            // No orders in orderbook
            if lmp != 0 {
                // Use lmp with spread (subtract spread for sell)
                let down = ((lmp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                let price = if lp <= down { down } else { lp };
                return (price, lmp);
            }
            // No lmp, return limit price as-is
            return (lp, lmp);
        } else if ask_head == 0 && bid_head != 0 {
            // Only bids exist
            if lmp != 0 {
                let mut down = ((lmp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                down = if lp <= down { down } else { lp };
                let price = if down <= bid_head { bid_head } else { down };
                return (price, lmp);
            }
            // No lmp, use bidHead with spread
            let mut down = ((bid_head as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
            down = if lp <= down { down } else { lp };
            let price = if down <= bid_head { bid_head } else { down };
            return (price, lmp);
        } else if ask_head != 0 && bid_head == 0 {
            // Only asks exist
            if lmp != 0 {
                let down = ((lmp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                let price = if lp <= down { down } else { lp };
                return (price, lmp);
            }
            // No lmp, use askHead with spread
            let down = ((ask_head as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
            let price = if lp <= down { down } else { lp };
            return (price, lmp);
        } else {
            // Both bids and asks exist
            if lmp != 0 {
                let down = ((lmp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                let price = if lp <= down { down } else { lp };
                return (price, lmp);
            }
            // No lmp, lower limit price on sell cannot be lower than bid head price
            // Note: Following Solidity code pattern, using lp directly when no lmp
            let price = if lp <= bid_head { bid_head } else { lp };
            return (price, lmp);
        }
    }

    /// Determine the maker price for a limit buy order
    /// This function calculates the price at which a limit buy order should be placed
    /// as a maker order based on current market conditions, limit price, and spread.
    /// 
    /// - `lp`: Limit price specified by the user
    /// - `bid_head`: Current best bid price (0 if no bids)
    /// - `ask_head`: Current best ask price (0 if no asks)
    /// - `spread`: Spread in basis points (10000 = 100%)
    /// - Returns the calculated maker price and lmp tuple
    #[cfg_attr(test, allow(dead_code))]
    pub fn det_limit_buy_make_price(
        &self,
        lp: u64,
        bid_head: u64,
        ask_head: u64,
        spread: u32,
    ) -> (u64, u64) {
        const DENOM: u64 = 10000; // Basis points denominator (10000 = 100%)
        let lmp = self.lmp().unwrap_or(0);

        if ask_head == 0 && bid_head == 0 {
            // No orders in orderbook
            if lmp != 0 {
                // Use lmp with spread (add spread for buy)
                let up = ((lmp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                let price = if lp >= up { up } else { lp };
                return (price, lmp);
            }
            // No lmp, return limit price as-is
            return (lp, lmp);
        } else if ask_head == 0 && bid_head != 0 {
            // Only bids exist
            if lmp != 0 {
                // Use lmp with spread (add spread for buy)
                let up = ((lmp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                let price = if lp >= up { up } else { lp };
                return (price, lmp);
            }
            // No lmp, use bidHead with spread
            let up = ((bid_head as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
            let price = if lp >= up { up } else { lp };
            return (price, lmp);
        } else if ask_head != 0 && bid_head == 0 {
            // Only asks exist
            if lmp != 0 {
                let mut up = ((lmp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                up = if lp >= up { up } else { lp };
                let price = if up >= ask_head { ask_head } else { up };
                return (price, lmp);
            }
            // No lmp, use askHead with spread
            let mut up = ((ask_head as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
            up = if lp >= up { up } else { lp };
            let price = if up >= ask_head { ask_head } else { up };
            return (price, lmp);
        } else {
            // Both bids and asks exist
            if lmp != 0 {
                let mut up = ((lmp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                up = if lp >= up { up } else { lp };
                let price = if up >= ask_head { ask_head } else { up };
                return (price, lmp);
            }
            // No lmp, upper limit on make price must not go above ask price
            let price = if lp >= ask_head { ask_head } else { lp };
            return (price, lmp);
        }
    }

    /// Determine the maker price for a market buy order
    /// This function calculates the price at which a market buy order should be placed
    /// as a maker order based on current market conditions (bid_head, ask_head, lmp) and spread.
    /// 
    /// - `bid_head`: Current best bid price (0 if no bids)
    /// - `ask_head`: Current best ask price (0 if no asks)
    /// - `spread`: Spread in basis points (10000 = 100%)
    /// - Returns the calculated maker price
    #[cfg_attr(test, allow(dead_code))]
    pub fn det_market_buy_make_price(
        &self,
        bid_head: u64,
        ask_head: u64,
        spread: u32,
    ) -> (u64, u64) {
        const DENOM: u64 = 10000; // Basis points denominator (10000 = 100%)
        let lmp = self.lmp().unwrap_or(0);

        if ask_head == 0 && bid_head == 0 {
            // No orders in orderbook
            // lmp must exist unless there has been no order in orderbook
            if lmp != 0 {
                // Use lmp with spread (add spread for buy)
                let up = ((lmp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                return (up, lmp);
            }
            // No lmp, return 0 (should not happen in practice)
            return (0, lmp);
        } else if ask_head == 0 && bid_head != 0 {
            // Only bids exist
            if lmp != 0 {
                let temp = if bid_head >= lmp { bid_head } else { lmp };
                let up = ((temp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                return (up, lmp);
            }
            // No lmp, use bid_head with spread
            let up = ((bid_head as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
            return (up, lmp);
        } else if ask_head != 0 && bid_head == 0 {
            // Only asks exist
            if lmp != 0 {
                let up = ((lmp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                let price = if ask_head >= up { up } else { ask_head };
                return (price, lmp);
            }
            // No lmp, return ask_head
            return (ask_head, lmp);
        } else {
            // Both bids and asks exist
            if lmp != 0 {
                let temp = if bid_head >= lmp { bid_head } else { lmp };
                let up = ((temp as u128 * (DENOM + spread as u64) as u128) / DENOM as u128) as u64;
                let price = if ask_head >= up { up } else { ask_head };
                return (price, lmp);
            }
            // No lmp, return ask_head
            return (ask_head, lmp);
        }
    }

    /// Determine the maker price for a market sell order
    /// This function calculates the price at which a market sell order should be placed
    /// as a maker order based on current market conditions (bid_head, ask_head, lmp) and spread.
    /// 
    /// - `bid_head`: Current best bid price (0 if no bids)
    /// - `ask_head`: Current best ask price (0 if no asks)
    /// - `spread`: Spread in basis points (10000 = 100%)
    /// - Returns the calculated maker price (minimum 1 to prevent zero price)
    #[cfg_attr(test, allow(dead_code))]
    pub fn det_market_sell_make_price(
        &self,
        bid_head: u64,
        ask_head: u64,
        spread: u32,
    ) -> (u64, u64) {
        const DENOM: u64 = 10000; // Basis points denominator (10000 = 100%)
        let lmp = self.lmp().unwrap_or(0);

        if ask_head == 0 && bid_head == 0 {
            // No orders in orderbook
            // lmp must exist unless there has been no order in orderbook
            if lmp != 0 {
                // Use lmp with spread (subtract spread for sell)
                let down = ((lmp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                // Ensure price is never 0
                let price = if down == 0 { 1 } else { down };
                return (price, lmp);
            }
            // No lmp, return 0 (should not happen in practice)
            return (0, lmp);
        } else if ask_head == 0 && bid_head != 0 {
            // Only bids exist
            if lmp != 0 {
                let mut down = ((lmp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                down = if down <= bid_head { bid_head } else { down };
                // Ensure price is never 0
                let price = if down == 0 { 1 } else { down };
                return (price, lmp);
            }
            // No lmp, return bid_head
            return (bid_head, lmp);
        } else if ask_head != 0 && bid_head == 0 {
            // Only asks exist
            if lmp != 0 {
                let temp = if lmp <= ask_head { lmp } else { ask_head };
                let down = ((temp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                // Ensure price is never 0
                let price = if down == 0 { 1 } else { down };
                return (price, lmp);
            }
            // No lmp, use ask_head with spread
            let down = ((ask_head as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
            // Ensure price is never 0
            let price = if down == 0 { 1 } else { down };
            return (price, lmp);
        } else {
            // Both bids and asks exist
            if lmp != 0 {
                let temp = if lmp <= ask_head { lmp } else { ask_head };
                let mut down = ((temp as u128 * (DENOM - spread as u64) as u128) / DENOM as u128) as u64;
                down = if down <= bid_head { bid_head } else { down };
                // Ensure price is never 0
                let price = if down == 0 { 1 } else { down };
                return (price, lmp);
            }
            // No lmp, return bid_head
            return (bid_head, lmp);
        }
    }

    
}
