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

impl Default for L1 {
    fn default() -> Self {
        Self {
            lmp: None,
            bid_head: None,
            ask_head: None,
            limit_buy_slippage_limit: None,
            limit_sell_slippage_limit: None,
            market_buy_slippage_limit: None,
            market_sell_slippage_limit: None,
        }
    }
}

impl L1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new() -> Self {
        Self {
            lmp: None,
            bid_head: None,
            ask_head: None,
            limit_buy_slippage_limit: None,
            limit_sell_slippage_limit: None,
            market_buy_slippage_limit: None,
            market_sell_slippage_limit: None,
        }
    }
}

