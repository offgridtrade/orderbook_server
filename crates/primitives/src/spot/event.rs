// core_events/src/lib.rs
use once_cell::sync::OnceCell;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::fmt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpotEvent {
    /// Pair added to the matching engine
    SpotPairAdded {
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// pair id
        pair_id: String,
        /// timestamp
        /// i64 is chosen because of js type compatibility
        timestamp: i64,
    },
    /// Transfer event from an account to another account
    Transfer {
        /// client id 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// from account id
        #[serde(with = "serde_bytes")]
        from: Vec<u8>,
        /// to account id
        #[serde(with = "serde_bytes")]
        to: Vec<u8>,
        /// asset id
        #[serde(with = "serde_bytes")]
        asset: Vec<u8>,
        /// amount
        amnt: u64,
        /// timestamp
        timestamp: i64,
    },
    /// Spot order block changed in the orderbook
    SpotOrderBlockChanged {
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// pair id
        #[serde(with = "serde_bytes")]
        pair_id: Vec<u8>,
        /// is bid
        is_bid: bool,
        /// price
        price: u64,
        /// amount
        amnt: u64,
        /// timestamp
        timestamp: i64,
    },
    /// Spot order placed in the orderbook being a maker
    SpotOrderPlaced { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64, 
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// current quantity
        cqty: u64, 
        /// public quantity
        pqty: u64,
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order matched in the orderbook being a taker
    SpotOrderMatched { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64,
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// taker account id
        #[serde(with = "serde_bytes")]
        taker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order partially matched in the orderbook being a taker
    SpotOrderPartiallyMatched { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64,
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// taker account id
        #[serde(with = "serde_bytes")]
        taker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order fully matched in the orderbook being a taker
    SpotOrderFullyMatched { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64,
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// taker account id
        #[serde(with = "serde_bytes")]
        taker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64,
        /// public quantity
        pqty: u64, 
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order cancelled in the orderbook regardless of being a maker or taker
    SpotOrderCancelled { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64, 
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order expired in the orderbook regardless of being a maker
    SpotOrderExpired { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64, 
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order filled in the orderbook regardless of being a maker
    SpotOrderFilled { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64, 
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64,
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order partially filled in the orderbook being a maker
    SpotOrderPartiallyFilled { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64, 
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
    /// Spot order fully filled in the orderbook being a maker
    SpotOrderFullyFilled { 
        /// client id
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        /// order id
        order_id: u64, 
        /// maker account id
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        /// is bid
        is_bid: bool, 
        /// price
        price: u64, 
        /// whole amount
        amnt: u64,
        /// iceberg quantity
        iqty: u64, 
        /// public quantity
        pqty: u64,
        /// current quantity
        cqty: u64, 
        /// timestamp, i64 is chosen because of js type compatibility
        timestamp: i64, 
        /// expires at timestamp, i64 is chosen because of js type compatibility
        expires_at: i64 
    },
}

/// A queue of events that can be formatted and displayed.
/// This is a newtype wrapper around `Vec<SpotEvent>` that provides better formatting support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventQueue(pub Vec<SpotEvent>);

impl EventQueue {
    /// Create a new empty event queue
    pub fn new() -> Self {
        EventQueue(Vec::new())
    }

    /// Create an event queue from a vector of events
    pub fn from_vec(events: Vec<SpotEvent>) -> Self {
        EventQueue(events)
    }

    /// Get a reference to the underlying vector
    pub fn as_vec(&self) -> &Vec<SpotEvent> {
        &self.0
    }

    /// Consume the wrapper and return the underlying vector
    pub fn into_vec(self) -> Vec<SpotEvent> {
        self.0
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the number of events in the queue
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        EventQueue::new()
    }
}

impl From<Vec<SpotEvent>> for EventQueue {
    fn from(events: Vec<SpotEvent>) -> Self {
        EventQueue(events)
    }
}

impl From<EventQueue> for Vec<SpotEvent> {
    fn from(queue: EventQueue) -> Self {
        queue.0
    }
}

impl fmt::Display for EventQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "[]");
        }
        
        write!(f, "[")?;
        for (i, event) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", event)?;
        }
        write!(f, "]")
    }
}

impl std::ops::Deref for EventQueue {
    type Target = Vec<SpotEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EventQueue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait EventBackend: Send + 'static {
    fn handle_event(&mut self, event: SpotEvent);
}

// Sender into the dispatcher
static DISPATCH_TX: OnceCell<mpsc::Sender<SpotEvent>> = OnceCell::new();

// List of per-backend senders
static BACKEND_TXS: OnceCell<Mutex<Vec<mpsc::Sender<SpotEvent>>>> = OnceCell::new();

// In-memory event queue that stores events before they are published
static EVENT_QUEUE: OnceCell<Mutex<Vec<SpotEvent>>> = OnceCell::new();

fn backend_txs() -> &'static Mutex<Vec<mpsc::Sender<SpotEvent>>> {
    BACKEND_TXS.get_or_init(|| Mutex::new(Vec::new()))
}

fn event_queue() -> &'static Mutex<Vec<SpotEvent>> {
    EVENT_QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Call once at process startup to create the dispatcher thread.
pub fn init_event_bus() {
    let (tx, rx) = mpsc::channel::<SpotEvent>();
    DISPATCH_TX.set(tx).ok(); // ignore if already set

    // Dispatcher thread: fan out every event to all registered backends.
    thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            // clone once per backend
            let backends = backend_txs().lock().unwrap();
            for backend_tx in backends.iter() {
                // Ignore send errors (backend might have shut down)
                let _ = backend_tx.send(event.clone());
            }
        }
    });
}

/// Called from anywhere (engine, core logic) to emit an event.
/// This stores the event in the event queue. Use `publish_events()` to actually send them.
pub fn emit_event(event: SpotEvent) {
    let mut queue = event_queue().lock().unwrap();
    queue.push(event);
}

/// Publishes all events from the global queue to the event bus (if initialized).
/// After publishing, the queue is drained and cleared.
pub fn publish_events() {
    // Drain all events from the queue
    let events: Vec<SpotEvent> = {
        let mut queue = event_queue().lock().unwrap();
        let drained = queue.clone();
        queue.clear();
        drained
    };

    // Send each event to the dispatcher if it's initialized
    if let Some(tx) = DISPATCH_TX.get() {
        for event in events {
            // ignore error if dispatcher is down
            let _ = tx.send(event);
        }
    }
}

/// Publishes an EventQueue to the event bus (if initialized).
/// This is useful when you have an EventQueue returned from an operation.
pub fn publish_event_queue(events: EventQueue) {
    // Send each event to the dispatcher if it's initialized
    if let Some(tx) = DISPATCH_TX.get() {
        for event in events.into_vec() {
            // ignore error if dispatcher is down
            let _ = tx.send(event);
        }
    }
}

/// Register a backend; returns an `mpsc::Receiver<SpotEvent>` that you
/// can consume from a dedicated thread.
pub fn register_backend() -> mpsc::Receiver<SpotEvent> {
    let (tx, rx) = mpsc::channel::<SpotEvent>();

    {
        let mut list = backend_txs().lock().unwrap();
        list.push(tx);
    }

    rx
}

/// Drains all events from the event queue and returns them.
/// This clears the queue after draining.
/// Useful for retrieving events after operations complete.
pub fn drain_events() -> EventQueue {
    let mut queue = event_queue().lock().unwrap();
    let drained = queue.clone();
    queue.clear();
    EventQueue(drained)
}

/// Clears all events from the event queue without returning them.
pub fn clear_events() {
    let mut queue = event_queue().lock().unwrap();
    queue.clear();
}
