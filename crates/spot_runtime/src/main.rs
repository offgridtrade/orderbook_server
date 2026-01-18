use offgrid_primitives::spot::MatchingEngine;
use offgrid_primitives::spot::event::{self, SpotEvent};
use offgrid_spot_runtime::{version, network as network_module, metrics, snapshot};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use zmq::Context;

fn main() -> anyhow::Result<()> {
    println!("Orderbook Server {} starting...", version());

    // Initialize event bus for event dispatching
    event::init_event_bus();
    println!("Event bus initialized");

    // Initialize ZMQ context
    let context = Context::new();

    // Get ports from environment or use defaults
    let (event_port, order_port) = network_module::get_ports()?;

    // Create ZMQ server
    let zmq_server = Arc::new(network_module::ZmqServer::new(&context, event_port, order_port)?);
    println!("ZMQ server initialized - Events: {}, Orders: {}", event_port, order_port);

    // Load matching engine from snapshot or create new
    let snapshot_path = std::env::var("SNAPSHOT_PATH")
        .unwrap_or_else(|_| "./data/snapshot.bin".to_string());
    
    println!("Loading matching engine from snapshot: {}", snapshot_path);
    let engine = match snapshot::load_snapshot_or_new(&snapshot_path) {
        Ok(engine) => {
            println!("Matching engine loaded: {} pairs", engine.pair_count());
            engine
        }
        Err(e) => {
            eprintln!("Warning: Failed to load snapshot ({}), starting with empty engine", e);
            MatchingEngine::new()
        }
    };
    
    // Create matching engine (shared across threads)
    let matching_engine = Arc::new(Mutex::new(engine));

    // Channel for events from order processing to event streaming thread
    let (event_tx, event_rx) = mpsc::channel::<Vec<u8>>();

    // Shutdown flag (shared across threads)
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Initialize Prometheus metrics (needed for metrics backend)
    let metrics_registry = Arc::new(metrics::Metrics::new()?);
    let metrics_port = metrics::get_metrics_port();

    // Register event backend #1: ZMQ event streaming
    let zmq_event_receiver = event::register_backend();
    let zmq_server_event_backend = zmq_server.clone();
    let shutdown_zmq_backend = shutdown_flag.clone();
    
    // Spawn thread to consume events from event bus and forward to ZMQ
    let zmq_event_backend_thread = thread::spawn(move || {
        println!("ZMQ event backend thread started");
        loop {
            if shutdown_zmq_backend.load(Ordering::Relaxed) {
                break;
            }
            
            match zmq_event_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    // Serialize event as JSON and send via ZMQ
                    // serde_bytes will automatically encode Vec<u8> as base64 strings in JSON
                    match serde_json::to_vec(&event) {
                        Ok(event_data) => {
                            if let Err(e) = zmq_server_event_backend.publish_event(&event_data) {
                                eprintln!("Error publishing event to ZMQ: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error serializing event to JSON: {}", e);
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    println!("ZMQ event backend channel disconnected");
                    break;
                }
            }
        }
        println!("ZMQ event backend thread stopped");
    });

    // Register event backend #2: Metrics
    let metrics_event_receiver = event::register_backend();
    let metrics_registry_for_events = metrics_registry.clone();
    let shutdown_metrics_backend = shutdown_flag.clone();
    
    // Spawn thread to consume events and update metrics
    let metrics_event_backend_thread = thread::spawn(move || {
        println!("Metrics event backend thread started");
        loop {
            if shutdown_metrics_backend.load(Ordering::Relaxed) {
                break;
            }
            
            match metrics_event_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    match event.clone() {
                        SpotEvent::SpotOrderPlaced { .. } => metrics_registry_for_events.orders_placed.inc(),
                        SpotEvent::SpotOrderMatched { .. } => metrics_registry_for_events.orders_matched.inc(),
                        SpotEvent::SpotOrderCancelled { .. } => metrics_registry_for_events.orders_cancelled.inc(),
                        SpotEvent::SpotOrderExpired { .. } => metrics_registry_for_events.orders_expired.inc(),
                        SpotEvent::SpotOrderFilled { .. } => metrics_registry_for_events.orders_filled.inc(),
                        SpotEvent::SpotOrderPartiallyFilled { .. } => metrics_registry_for_events.orders_partially_filled.inc(),
                        SpotEvent::SpotOrderFullyFilled { .. } => metrics_registry_for_events.orders_fully_filled.inc(),
                        SpotEvent::Transfer { .. } => {}
                        SpotEvent::SpotOrderBlockChanged { .. } => {}
                        SpotEvent::SpotOrderPartiallyMatched { .. } => {}
                        SpotEvent::SpotOrderFullyMatched { .. } => {}
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    println!("Metrics event backend channel disconnected");
                    break;
                }
            }
        }
        println!("Metrics event backend thread stopped");
    });

    // Register event backend #3: Logging
    let logging_event_receiver = event::register_backend();
    let shutdown_logging_backend = shutdown_flag.clone();
    
    // Spawn thread to consume events and log them
    let logging_event_backend_thread = thread::spawn(move || {
        println!("Logging event backend thread started");
        loop {
            if shutdown_logging_backend.load(Ordering::Relaxed) {
                break;
            }
            
            match logging_event_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    // Log the event
                    // TODO: Use proper structured logging library
                    println!("[EVENT] {:?}", event);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    println!("Logging event backend channel disconnected");
                    break;
                }
            }
        }
        println!("Logging event backend thread stopped");
    });

    // Spawn event streaming thread (for raw order data)
    let event_thread = network_module::spawn_event_streaming_thread(
        zmq_server.clone(),
        event_rx,
        shutdown_flag.clone(),
    );

    // Spawn snapshot thread (saves state periodically)
    let snapshot_interval = std::env::var("SNAPSHOT_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60); // Default: 60 seconds
    
    let snapshot_thread = snapshot::spawn_snapshot_thread(
        matching_engine.clone(),
        snapshot_path.clone(),
        snapshot_interval,
        shutdown_flag.clone(),
    );

    // Spawn cron jobs thread
    // TODO: Update to use matching_engine instead of orderbook
    // let cron_thread = jobs::spawn_cron_thread(
    //     matching_engine.clone(),
    //     shutdown_flag.clone(),
    // );

    // Spawn Prometheus metrics HTTP server thread
    let metrics_thread = metrics::spawn_metrics_thread(
        metrics_registry.clone(),
        shutdown_flag.clone(),
        metrics_port,
    );
    println!("Prometheus metrics server started on port {}", metrics_port);

    // Main thread: order processing from gateway using ROUTER socket
    println!("Main order processing thread started");
    
    // Get reference to the router socket from zmq_server
    let order_router = zmq_server.order_router();
    
    // Set up signal handler for graceful shutdown
    let shutdown_main = shutdown_flag.clone();
    ctrlc::set_handler(move || {
        println!("\nShutdown signal received, cleaning up...");
        shutdown_main.store(true, Ordering::Relaxed);
    })?;

    // Main order processing loop
    loop {
        // Check for shutdown signal
        if shutdown_flag.load(Ordering::Relaxed) {
            println!("Shutdown signal received in main thread");
            break;
        }

        // Poll for incoming orders (non-blocking)
        let mut items = [order_router.as_poll_item(zmq::POLLIN)];
        match zmq::poll(&mut items, 100)? {
            0 => continue, // Timeout, continue loop
            _ => {
                // Receive order message from DEALER client
                if let Some((identity, msg)) = network_module::receive_order(order_router) {
                    // Process order
                    let order_data = msg.to_vec();
                    
                    // TODO: Parse order message and process through matching engine
                    // Example:
                    // let order_result = {
                    //     let mut engine = matching_engine_main.lock().unwrap();
                    //     engine.limit_buy(...)?;
                    // };
                    
                    // Send event to event streaming thread
                    if let Err(e) = event_tx.send(order_data.to_vec()) {
                        eprintln!("Error sending event: {}", e);
                    }
                    
                    // Send acknowledgment back to gateway via ROUTER
                    if let Err(e) = network_module::send_ack(order_router, &identity, "ACK") {
                        eprintln!("Error sending ack: {}", e);
                    }
                }
            }
        }
    }

    // Wait for all threads to finish
    println!("Waiting for threads to finish...");
    let _ = event_thread.join();
    let _ = zmq_event_backend_thread.join();
    let _ = metrics_event_backend_thread.join();
    let _ = logging_event_backend_thread.join();
    // let _ = cron_thread.join();
    let _ = snapshot_thread.join();
    let _ = metrics_thread.join();

    println!("Orderbook Server shutdown complete");
    Ok(())
}
