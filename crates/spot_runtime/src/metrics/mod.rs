use prometheus::{Encoder, Registry, TextEncoder};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Prometheus metrics registry
pub struct Metrics {
    pub registry: Registry,
    pub transfers_total: prometheus::IntCounter,
    pub orders_placed: prometheus::IntCounter,
    pub orders_partially_matched: prometheus::IntCounter,
    pub orders_fully_matched: prometheus::IntCounter,
    pub orders_cancelled: prometheus::IntCounter,
    pub orders_expired: prometheus::IntCounter,
    pub order_iceberg_quantity_changed: prometheus::IntCounter,
    pub orders_partially_filled: prometheus::IntCounter,
    pub orders_fully_filled: prometheus::IntCounter,
    pub orderbook_depth_bid: prometheus::IntGauge,
    pub orderbook_depth_ask: prometheus::IntGauge,
    pub order_processing_duration: prometheus::Histogram,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        // Define metrics
        let transfers_total =
            prometheus::IntCounter::new("orderbook_transfers_total", "Total number of transfers")?;
        let orders_placed = prometheus::IntCounter::new(
            "orderbook_orders_placed_total",
            "Total number of orders placed",
        )?;
        let orders_partially_matched = prometheus::IntCounter::new(
            "orderbook_orders_partially_matched_total",
            "Total number of orders partially matched",
        )?;
        let orders_fully_matched = prometheus::IntCounter::new(
            "orderbook_orders_fully_matched_total",
            "Total number of orders fully matched",
        )?;
        let orders_cancelled = prometheus::IntCounter::new(
            "orderbook_orders_cancelled_total",
            "Total number of orders cancelled",
        )?;
        let orders_expired = prometheus::IntCounter::new(
            "orderbook_orders_expired_total",
            "Total number of orders expired",
        )?;
        let order_iceberg_quantity_changed = prometheus::IntCounter::new(
            "orderbook_order_iceberg_quantity_changed_total",
            "Total number of iceberg quantity changes",
        )?;
        let orders_partially_filled = prometheus::IntCounter::new(
            "orderbook_orders_partially_filled_total",
            "Total number of orders partially filled",
        )?;
        let orders_fully_filled = prometheus::IntCounter::new(
            "orderbook_orders_fully_filled_total",
            "Total number of orders fully filled",
        )?;
        let orderbook_depth_bid = prometheus::IntGauge::new(
            "orderbook_depth_bid",
            "Current depth of bid side orderbook",
        )?;
        let orderbook_depth_ask = prometheus::IntGauge::new(
            "orderbook_depth_ask",
            "Current depth of ask side orderbook",
        )?;
        let order_processing_duration = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "orderbook_order_processing_duration_seconds",
                "Time spent processing orders",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        )?;

        // Register metrics
        registry.register(Box::new(transfers_total.clone()))?;
        registry.register(Box::new(orders_placed.clone()))?;
        registry.register(Box::new(orders_partially_matched.clone()))?;
        registry.register(Box::new(orders_fully_matched.clone()))?;
        registry.register(Box::new(orders_cancelled.clone()))?;
        registry.register(Box::new(orders_expired.clone()))?;
        registry.register(Box::new(order_iceberg_quantity_changed.clone()))?;
        registry.register(Box::new(orders_partially_filled.clone()))?;
        registry.register(Box::new(orders_fully_filled.clone()))?;
        registry.register(Box::new(orderbook_depth_bid.clone()))?;
        registry.register(Box::new(orderbook_depth_ask.clone()))?;
        registry.register(Box::new(order_processing_duration.clone()))?;

        Ok(Self {
            registry,
            transfers_total,
            orders_placed,
            orders_partially_matched,
            orders_fully_matched,
            orders_cancelled,
            orders_expired,
            order_iceberg_quantity_changed,
            orders_partially_filled,
            orders_fully_filled,
            orderbook_depth_bid,
            orderbook_depth_ask,
            order_processing_duration,
        })
    }
}

/// Spawn Prometheus metrics HTTP server thread
pub fn spawn_metrics_thread(
    metrics: Arc<Metrics>,
    shutdown_flag: Arc<AtomicBool>,
    port: u16,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("Prometheus metrics thread started on port {}", port);

        let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind metrics server on port {}: {}", port, e);
                return;
            }
        };

        // Set non-blocking mode
        listener
            .set_nonblocking(true)
            .expect("Failed to set non-blocking");

        loop {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }

            match listener.accept() {
                Ok((mut stream, _)) => {
                    // Set read timeout for the stream
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .expect("Failed to set read timeout");
                    handle_metrics_request(&mut stream, &metrics);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection available, continue loop
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    eprintln!("Error accepting metrics connection: {}", e);
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }

        println!("Prometheus metrics thread stopped");
    })
}

fn handle_metrics_request(stream: &mut TcpStream, metrics: &Metrics) {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer);

    let request = String::from_utf8_lossy(&buffer);
    let response = if request.starts_with("GET /metrics") {
        let encoder = TextEncoder::new();
        let metric_families = metrics.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
            encoder.format_type(),
            buffer.len(),
            String::from_utf8_lossy(&buffer)
        )
    } else if request.starts_with("GET /health") {
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK".to_string()
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found".to_string()
    };

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Get metrics port from environment variable or use default
pub fn get_metrics_port() -> u16 {
    std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse::<u16>()
        .unwrap_or(9090)
}
