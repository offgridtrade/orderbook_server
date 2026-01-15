# Offgrid Spot Runtime

Offgrid Runtime is the in-memory application layer for the Offgrid protocol. It handles business logic from Offgrid team's onchain applications, including:

- **Spot Exchange** - Real-time order matching and trade execution
- **Futures** - Futures contract trading and settlement
- **Options** - Options contract trading and exercise
- **Money Market** - Lending and borrowing operations

## Features

- High-performance in-memory orderbook with L1/L2/L3 data structures
- Event-driven architecture with ZMQ pub/sub for real-time event streaming
- Prometheus metrics integration for monitoring
- Automatic state snapshots for persistence and recovery
- Multi-threaded architecture for concurrent order processing

## Prerequisites

- **Rust** (latest stable version, recommended 1.70+)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **ZeroMQ** development libraries
  - **Ubuntu/Debian**: `sudo apt-get install libzmq3-dev`
  - **macOS**: `brew install zeromq`
  - **Fedora**: `sudo dnf install zeromq-devel`
- **RocksDB** (for optional database features)
  - **Ubuntu/Debian**: `sudo apt-get install librocksdb-dev`
  - **macOS**: `brew install rocksdb`
  - **Fedora**: `sudo dnf install rocksdb-devel`

## Building

### From the workspace root:

```bash
# Build the runtime in release mode
cargo build --release -p offgrid-spot-runtime

# Or build from the runtime directory
cd crates/spot_runtime
cargo build --release
```

### Build artifacts:

The compiled binary will be located at:
```
target/release/offgrid-spot-runtime
```

## Configuration

The runtime can be configured using environment variables:

### Network Ports

- `EVENT_PORT` - Port for ZMQ PUB socket (event streaming)
  - Default: `5555`
- `ORDER_PORT` - Port for ZMQ ROUTER socket (order processing)
  - Default: `5556`
- `METRICS_PORT` - Port for Prometheus metrics HTTP server
  - Default: `9090`

### State Management

- `SNAPSHOT_PATH` - Path to save/load state snapshots
  - Default: `./data/snapshot.bin`
- `SNAPSHOT_INTERVAL_SECONDS` - Interval between automatic snapshots
  - Default: `60` seconds

### Example Configuration

```bash
export EVENT_PORT=5555
export ORDER_PORT=5556
export METRICS_PORT=9090
export SNAPSHOT_PATH=./data/snapshot.bin
export SNAPSHOT_INTERVAL_SECONDS=60
```

## Running

### Basic Usage

```bash
# Run from workspace root
cargo run --release -p offgrid-spot-runtime

# Or run from runtime directory
cd crates/spot_runtime
cargo run --release
```

### With Custom Configuration

```bash
EVENT_PORT=5555 \
ORDER_PORT=5556 \
METRICS_PORT=9090 \
SNAPSHOT_PATH=./data/snapshot.bin \
SNAPSHOT_INTERVAL_SECONDS=60 \
cargo run --release -p offgrid-spot-runtime
```

### Production Deployment

```bash
# Build release binary
cargo build --release -p offgrid-spot-runtime

# Run the binary
./target/release/offgrid-spot-runtime
```

## Architecture

### Thread Model

The runtime uses a multi-threaded architecture:

1. **Main Thread** - Handles order processing from gateway via ZMQ ROUTER
2. **Event Dispatcher Thread** - Fans out events to all registered backends
3. **ZMQ Event Backend Thread** - Streams events to subscribers via PUB socket
4. **Metrics Backend Thread** - Updates Prometheus metrics
5. **Logging Backend Thread** - Logs events for debugging
6. **Snapshot Thread** - Periodically saves state to disk
7. **Metrics HTTP Server Thread** - Serves Prometheus metrics endpoint

### Event Flow

```
Order Processing → MatchingEngine → EventQueue → publish_event_queue()
                                                      ↓
                                              Event Dispatcher
                                                      ↓
                    ┌─────────────────────────────────┼─────────────────────────────────┐
                    ↓                                 ↓                                 ↓
            ZMQ Backend Thread              Metrics Backend Thread          Logging Backend Thread
                    ↓                                 ↓                                 ↓
            Gateway (PUB socket)            Prometheus Metrics              Console Logs
```

## Monitoring

### Prometheus Metrics

Access metrics at: `http://localhost:9090/metrics`

Available metrics:
- `orders_placed` - Total orders placed
- `orders_matched` - Total orders matched
- `orders_cancelled` - Total orders cancelled
- `orders_filled` - Total orders filled
- `orders_partially_filled` - Total partially filled orders
- `orders_fully_filled` - Total fully filled orders
- `orders_expired` - Total expired orders

### Health Checks

The runtime responds to shutdown signals (SIGINT, SIGTERM) gracefully:
- Saves final snapshot before shutdown
- Waits for all threads to complete
- Closes network connections cleanly

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for runtime only
cargo test -p offgrid-runtime

# Run tests with output
cargo test -- --nocapture
```

### Code Structure

```
crates/runtime/
├── src/
│   ├── main.rs           # Entry point and thread orchestration
│   ├── lib.rs            # Library exports
│   ├── network/          # ZMQ networking layer
│   ├── metrics/          # Prometheus metrics
│   ├── snapshot.rs       # State persistence
│   └── jobs/             # Background jobs (cron tasks)
├── proto/                # Protocol buffer definitions
└── build.rs              # Build script for proto compilation
```

## Troubleshooting

### Port Already in Use

If you see "Address already in use" errors:
- Check if another instance is running: `lsof -i :5555`
- Change ports using environment variables
- Kill existing process: `kill -9 <PID>`

### Missing Dependencies

If build fails with missing system libraries:
- Ensure ZeroMQ development headers are installed
- On Linux, you may need `pkg-config` for library detection
- Verify with: `pkg-config --modversion libzmq`

### Snapshot Errors

If snapshot save/load fails:
- Ensure the directory exists: `mkdir -p ./data`
- Check file permissions
- Verify disk space availability

## License

[Add your license information here]
