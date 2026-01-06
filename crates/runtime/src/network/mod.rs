use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use zmq::{Context, Socket, PUB, ROUTER};

/// ZMQ server for handling event streaming and order processing
pub struct ZmqServer {
    event_pub: Socket,
    order_router: Socket,
}

// `zmq::Socket` uses an internal raw pointer (`*mut c_void`) and is not marked
// `Send`/`Sync` by default, so wrapping it in a struct behind `Arc` trips the
// thread-safety bounds on `thread::spawn`. In this codebase each socket is
// confined to a single thread (PUB used only in the event thread, ROUTER only
// in the main thread), so it is safe to treat `ZmqServer` as `Send + Sync`.
// We declare that explicitly here so `Arc<ZmqServer>` can be moved into
// spawned threads.
unsafe impl Send for ZmqServer {}
unsafe impl Sync for ZmqServer {}

impl ZmqServer {
    /// Create a new ZMQ server with PUB socket for events and ROUTER socket for orders
    pub fn new(context: &Context, event_port: u16, order_port: u16) -> Result<Self> {
        // Create PUB socket for event streaming
        let event_pub = context.socket(PUB)?;
        event_pub.bind(&format!("tcp://*:{}", event_port))?;

        // Create ROUTER socket for order processing (dealer pattern)
        let order_router = context.socket(ROUTER)?;
        order_router.bind(&format!("tcp://*:{}", order_port))?;

        Ok(Self {
            event_pub,
            order_router,
        })
    }

    /// Publish an event to subscribers via PUB socket
    pub fn publish_event(&self, event: &[u8]) -> Result<()> {
        self.event_pub.send(event, 0)?;
        Ok(())
    }

    /// Get a reference to the order router socket
    pub fn order_router(&self) -> &Socket {
        &self.order_router
    }
}

/// Spawn a thread that streams events via ZMQ PUB socket
pub fn spawn_event_streaming_thread(
    zmq_server: Arc<ZmqServer>,
    event_rx: mpsc::Receiver<Vec<u8>>,
    shutdown_flag: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
        thread::spawn(move || {
        println!("Event streaming thread started");
        loop {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            
            match event_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event_data) => {
                    if let Err(e) = zmq_server.publish_event(&event_data) {
                        eprintln!("Error publishing event: {}", e);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Continue loop, check for shutdown
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    println!("Event channel disconnected, shutting down event thread");
                    break;
                }
            }
        }
        println!("Event streaming thread stopped");
    })
}

/// Receive an order message from the ROUTER socket
/// Returns (identity, message) if successful, None otherwise
pub fn receive_order(order_router: &Socket) -> Option<(zmq::Message, zmq::Message)> {
    let mut identity = zmq::Message::new();
    let mut empty = zmq::Message::new();
    let mut msg = zmq::Message::new();
    
    if order_router.recv(&mut identity, zmq::DONTWAIT).is_ok() &&
       order_router.recv(&mut empty, zmq::DONTWAIT).is_ok() &&
       order_router.recv(&mut msg, zmq::DONTWAIT).is_ok() {
        Some((identity, msg))
    } else {
        None
    }
}

/// Send an acknowledgment back to the client via ROUTER socket
pub fn send_ack(order_router: &Socket, identity: &zmq::Message, ack: &str) -> Result<()> {
    // ROUTER socket sends: [identity, empty, message]
    // send the identity frame as raw bytes so ZMQ can route back to the client
    order_router.send(identity.as_ref(), zmq::SNDMORE)?;
    order_router.send(&[] as &[u8], zmq::SNDMORE)?;
    order_router.send(ack.as_bytes(), 0)?;
    Ok(())
}

/// Get default ports from environment variables or use defaults
pub fn get_ports() -> Result<(u16, u16)> {
    let event_port = std::env::var("EVENT_PORT")
        .unwrap_or_else(|_| "5555".to_string())
        .parse::<u16>()?;
    let order_port = std::env::var("ORDER_PORT")
        .unwrap_or_else(|_| "5556".to_string())
        .parse::<u16>()?;
    Ok((event_port, order_port))
}

