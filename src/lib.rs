pub mod proto;
pub mod snapshot;
pub mod primitives;

use std::sync::mpsc::{Receiver, TryRecvError};

use anyhow::Context;

pub use primitives::{L1Struct as L1, L2Struct as L2, Level, OrderStorage as L3OrderStorage};

/// Runs a ZeroMQ ROUTER server that echoes messages back to the sending identity.
///
/// The server binds to the provided `bind_endpoint` (for example, `tcp://127.0.0.1:5555`)
/// and keeps running until a message is received on the `shutdown` channel.
pub fn run_router_server(bind_endpoint: &str, shutdown: Receiver<()>) -> anyhow::Result<()> {
    let context = zmq::Context::new();
    let socket = context
        .socket(zmq::ROUTER)
        .context("failed to create ROUTER socket")?;
    socket
        .bind(bind_endpoint)
        .with_context(|| format!("failed to bind ROUTER socket to {bind_endpoint}"))?;

    println!("ZeroMQ ROUTER server bound to {bind_endpoint}");

    loop {
        match shutdown.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                println!("Shutdown signal received. Stopping ROUTER server.");
                break;
            }
            Err(TryRecvError::Empty) => {
                // No shutdown signal yet; continue processing messages.
            }
        }

        let mut poll_items = [socket.as_poll_item(zmq::POLLIN)];
        zmq::poll(&mut poll_items, 100).context("polling ROUTER socket failed")?;

        if poll_items[0].is_readable() {
            let multipart = socket
                .recv_multipart(0)
                .context("failed to receive multipart message")?;
            println!("Received multipart message with {} frames", multipart.len());

            socket
                .send_multipart(multipart, 0)
                .context("failed to echo multipart message")?;
        }
    }

    Ok(())
}
