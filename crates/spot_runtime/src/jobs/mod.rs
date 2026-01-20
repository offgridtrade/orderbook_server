use offgrid_primitives::spot::orderbook::OrderBook;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// Spawn a cron jobs thread that runs periodic tasks
pub fn spawn_cron_thread(
    orderbook: Arc<Mutex<OrderBook>>,
    shutdown_flag: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("Cron jobs thread started");
        let interval = Duration::from_secs(60); // Run every minute
        loop {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            
            thread::sleep(interval);
            
            // Run cron jobs
            if let Ok(mut ob) = orderbook.lock() {
                cleanup_expired_orders(&mut ob);
            }
        }
        println!("Cron jobs thread stopped");
    })
}

/// Clean up expired orders from the orderbook.
///
/// This uses the underlying `expire_orders` API on the `OrderBook`, which:
/// - Scans L3 for orders whose `expires_at` is before `now`
/// - Removes them from L3/L2
/// - Emits `SpotOrderExpired` and corresponding `Transfer` events
fn cleanup_expired_orders(orderbook: &mut OrderBook) {
    // Current UNIX timestamp in milliseconds
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Runtime does not yet track concrete pair / asset / managing account ids here,
    // so we use placeholder identifiers. Downstream consumers can treat these as
    // "generic" housekeeping events.
    let pair_id = b"default_pair".to_vec();
    let base_asset_id = b"base".to_vec();
    let quote_asset_id = b"quote".to_vec();
    let managing_account_id = b"manager".to_vec();

    // Expire resting bid orders
    if let Err(e) = orderbook.expire_orders(
        true,
        pair_id.clone(),
        base_asset_id.clone(),
        quote_asset_id.clone(),
        managing_account_id.clone(),
        now,
    ) {
        eprintln!("Error expiring bid orders in cron job: {:?}", e);
    }

    // Expire resting ask orders
    if let Err(e) = orderbook.expire_orders(
        false,
        pair_id,
        base_asset_id,
        quote_asset_id,
        managing_account_id,
        now,
    ) {
        eprintln!("Error expiring ask orders in cron job: {:?}", e);
    }
}

