use once_cell::sync::Lazy;
use std::sync::Mutex;

pub mod limit_order;
pub mod market_order;
pub mod snapshot;

pub(crate) static EVENT_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
