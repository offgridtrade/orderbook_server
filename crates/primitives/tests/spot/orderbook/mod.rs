use once_cell::sync::Lazy;
use std::sync::Mutex;

pub(crate) static EVENT_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

mod snapshot;
mod order_placement;
mod trading;
