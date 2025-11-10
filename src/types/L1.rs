struct L1 {
    /// Last match price
    lmp: u64,
    /// Head of the bid list
    bidHead: u64,
    /// Head of the ask list
    askHead: u64,
    /// Slippage limit for limit buy orders in 8 decimals
    limit_buy_slippage_limit: u64,
    /// Slippage limit for limit sell orders in 8 decimals
    limit_sell_slippage_limit: u64,
    /// Slippage limit for market buy orders in 8 decimals
    market_buy_slippage_limit: u64,
    /// Slippage limit for market sell orders in 8 decimals
    market_sell_slippage_limit: u64,
}