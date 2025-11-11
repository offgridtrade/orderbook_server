use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L1 {
    /// Last match price
    pub lmp: u64,
    /// Head of the bid list
    pub bid_head: u64,
    /// Head of the ask list
    pub ask_head: u64,
    /// Slippage limit for limit buy orders in 8 decimals
    pub limit_buy_slippage_limit: u64,
    /// Slippage limit for limit sell orders in 8 decimals
    pub limit_sell_slippage_limit: u64,
    /// Slippage limit for market buy orders in 8 decimals
    pub market_buy_slippage_limit: u64,
    /// Slippage limit for market sell orders in 8 decimals
    pub market_sell_slippage_limit: u64,
}

impl L1 {
    pub fn new(
        lmp: u64,
        bid_head: u64,
        ask_head: u64,
        limit_buy_slippage_limit: u64,
        limit_sell_slippage_limit: u64,
        market_buy_slippage_limit: u64,
        market_sell_slippage_limit: u64,
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
}
