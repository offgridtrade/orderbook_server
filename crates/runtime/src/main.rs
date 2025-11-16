use offgrid_primitives::orderbook::OrderBook;
use offgrid_primitives::event::{self, Event};
use offgrid_runtime::{version, network as network_module, jobs, metrics};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
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

    // Create orderbook (shared across threads)
    let orderbook = Arc::new(std::sync::Mutex::new(OrderBook::new()));

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
                    // Serialize event and send via ZMQ
                    // TODO: Use proper serialization (bincode, prost, etc.)
                    let event_data = format!("{:?}", event).into_bytes();
                    if let Err(e) = zmq_server_event_backend.publish_event(&event_data) {
                        eprintln!("Error publishing event to ZMQ: {}", e);
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
                    match event {
                        Event::OrderPlaced { .. } => {
                            metrics_registry_for_events.orders_placed.inc();
                        }
                        Event::OrderMatched { .. } => {
                            metrics_registry_for_events.orders_matched.inc();
                        }
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

    // Spawn cron jobs thread
    let cron_thread = jobs::spawn_cron_thread(
        orderbook.clone(),
        shutdown_flag.clone(),
    );

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
    let orderbook_main = orderbook.clone();
    
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
                    let order_data = msg.as_slice();
                    
                    // TODO: Parse order message and process through orderbook
                    // Example:
                    // let order_result = {
                    //     let mut ob = orderbook_main.lock().unwrap();
                    //     ob.place_bid(...)?;
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
    let _ = cron_thread.join();
    let _ = metrics_thread.join();

    println!("Orderbook Server shutdown complete");
    Ok(())
}
