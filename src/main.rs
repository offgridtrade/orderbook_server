use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use anyhow::Context;

use orderbook_server::{
    L1, L2, L3OrderStorage, Level,
    snapshot::{LevelsSnapshotSource, SnapshotCron},
};

fn main() -> anyhow::Result<()> {
    let bind_endpoint =
        std::env::var("ROUTER_BIND").unwrap_or_else(|_| "tcp://127.0.0.1:5555".to_string());

    let snapshot_dir = std::env::var("SNAPSHOT_DIR").unwrap_or_else(|_| "snapshots".to_string());
    let snapshot_interval = std::env::var("SNAPSHOT_INTERVAL_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(10));

    let provider = Arc::new(InMemorySnapshotSource::new());
    let mut snapshot_cron = SnapshotCron::start(snapshot_dir.clone(), snapshot_interval, provider);
    println!(
        "Snapshot cron started. Persisting levels to `{snapshot_dir}` every {:?}.",
        snapshot_interval
    );

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let server_endpoint = bind_endpoint.clone();

    let server_handle = thread::spawn(move || {
        if let Err(err) = orderbook_server::run_router_server(&server_endpoint, shutdown_rx) {
            eprintln!("Router server exited with error: {err:?}");
        }
    });

    let shutdown_trigger = shutdown_tx.clone();
    ctrlc::set_handler(move || {
        println!("Ctrl+C received. Initiating shutdown...");
        let _ = shutdown_trigger.send(());
    })
    .context("failed to install Ctrl+C handler")?;

    println!("ZeroMQ ROUTER server listening on {bind_endpoint}");
    println!("Press Ctrl+C to stop the server.");

    server_handle
        .join()
        .expect("router server thread panicked unexpectedly");

    snapshot_cron.stop();

    // Ensure the main thread notifies the server to shut down if it hasn't already.
    let _ = shutdown_tx.send(());

    Ok(())
}

struct InMemorySnapshotSource {
    l1: Mutex<L1>,
    l2: Mutex<L2>,
    l3: Mutex<L3OrderStorage>,
}

impl InMemorySnapshotSource {
    fn new() -> Self {
        let l1 = L1::new(100, 95, 105, 5, 5, 10, 10);

        let l2 = L2 {
            bids: vec![Level {
                price: 100,
                quantity: 1_000,
            }],
            asks: vec![Level {
                price: 105,
                quantity: 1_500,
            }],
            ..Default::default()
        };

        let mut l3 = L3OrderStorage::new();
        let (id, _) = l3
            .create_order(vec![1], vec![2], 100, 10, 10, 0)
            .expect("create order for snapshot");
        l3.insert_id(100, id, 10)
            .expect("insert order into snapshot storage");

        Self {
            l1: Mutex::new(l1),
            l2: Mutex::new(l2),
            l3: Mutex::new(l3),
        }
    }
}

impl LevelsSnapshotSource for InMemorySnapshotSource {
    fn snapshot_levels(&self) -> (L1, L2, L3OrderStorage) {
        let l1 = self.l1.lock().expect("lock l1").clone();
        let l2 = self.l2.lock().expect("lock l2").clone();
        let l3 = self.l3.lock().expect("lock l3").clone();
        (l1, l2, l3)
    }
}
