use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Context;

fn next_tcp_endpoint() -> anyhow::Result<String> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .context("failed to acquire an ephemeral tcp port")?;
    let address = listener.local_addr().context("failed to read local addr")?;
    drop(listener);
    Ok(format!("tcp://{}:{}", address.ip(), address.port()))
}

#[test]
fn dealer_can_roundtrip_through_router() -> anyhow::Result<()> {
    let endpoint = next_tcp_endpoint()?;
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let server_endpoint = endpoint.clone();
    let server_handle = thread::spawn(move || {
        orderbook_server::run_router_server(&server_endpoint, shutdown_rx)
            .expect("router server failed");
    });

    // Give the router a moment to bind before the client connects.
    thread::sleep(Duration::from_millis(200));

    let context = zmq::Context::new();
    let dealer = context
        .socket(zmq::DEALER)
        .context("failed to create DEALER socket")?;
    let identity = format!("client-{}", std::process::id());
    dealer
        .set_identity(identity.as_bytes())
        .context("failed to set DEALER identity")?;
    dealer
        .connect(&endpoint)
        .with_context(|| format!("failed to connect DEALER to {endpoint}"))?;

    dealer.send("PING", 0).context("failed to send PING")?;
    let response = dealer
        .recv_string(0)
        .context("failed to receive PING echo")?
        .expect("router responded with non-UTF8 data");

    assert_eq!(response, "PING");

    shutdown_tx.send(()).ok();
    server_handle.join().expect("router server thread panicked");

    Ok(())
}
