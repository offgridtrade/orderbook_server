use serde::{Deserialize, Serialize};
/// Time in force (TIF) specifies how long an order should remain active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Fill or Kill (FOK): Order must be filled completely immediately or canceled
    /// If the order cannot be fully filled immediately, it is rejected/canceled
    FillOrKill,
    /// Immediate or Cancel (IOC): Order can be partially filled immediately, remaining is canceled
    /// Any portion that cannot be filled immediately is canceled
    ImmediateOrCancel,
    /// Good Till Canceled (GTC): Order stays in the orderbook until filled or manually canceled
    /// This is the default behavior for limit orders
    GoodTillCanceled,
}

impl Default for TimeInForce {
    fn default() -> Self {
        TimeInForce::GoodTillCanceled
    }
}