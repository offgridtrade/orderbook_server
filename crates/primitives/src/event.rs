// core_events/src/lib.rs
use once_cell::sync::OnceCell;
use std::sync::{mpsc, Mutex};
use std::thread;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    OrderPlaced { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64, 
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
    OrderMatched { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64,
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        #[serde(with = "serde_bytes")]
        taker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
    OrderCancelled { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64, 
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
    OrderExpired { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64, 
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
    OrderFilled { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64, 
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
    OrderPartiallyFilled { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64, 
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
    OrderFullyFilled { 
        #[serde(with = "serde_bytes")]
        cid: Vec<u8>,
        order_id: u64, 
        #[serde(with = "serde_bytes")]
        maker_account_id: Vec<u8>, 
        is_bid: bool, 
        price: u64, 
        iqty: u64, 
        cqty: u64, 
        timestamp: i64, 
        expires_at: i64 
    },
}

pub trait EventBackend: Send + 'static {
    fn handle_event(&mut self, event: Event);
}

// Sender into the dispatcher
static DISPATCH_TX: OnceCell<mpsc::Sender<Event>> = OnceCell::new();

// List of per-backend senders
static BACKEND_TXS: OnceCell<Mutex<Vec<mpsc::Sender<Event>>>> = OnceCell::new();

fn backend_txs() -> &'static Mutex<Vec<mpsc::Sender<Event>>> {
    BACKEND_TXS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Call once at process startup to create the dispatcher thread.
pub fn init_event_bus() {
    let (tx, rx) = mpsc::channel::<Event>();
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
pub fn emit_event(event: Event) {
    if let Some(tx) = DISPATCH_TX.get() {
        // ignore error if dispatcher is down
        let _ = tx.send(event);
    } else {
        // Optional: panic or log
        // eprintln!("Event bus not initialized");
    }
}

/// Register a backend; returns an `mpsc::Receiver<Event>` that you
/// can consume from a dedicated thread.
pub fn register_backend() -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel::<Event>();

    {
        let mut list = backend_txs().lock().unwrap();
        list.push(tx);
    }

    rx
}
