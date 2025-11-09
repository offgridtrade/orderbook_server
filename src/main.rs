use std::sync::mpsc;
use std::thread;

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let bind_endpoint =
        std::env::var("ROUTER_BIND").unwrap_or_else(|_| "tcp://127.0.0.1:5555".to_string());

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

    // Ensure the main thread notifies the server to shut down if it hasn't already.
    let _ = shutdown_tx.send(());

    Ok(())
}
