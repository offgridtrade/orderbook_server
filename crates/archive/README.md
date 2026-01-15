# Offgrid Archive

Offgrid Archive node manages account state from runtime results and stores transactions and core app events. Archive nodes are then used to submit state changes to L1 or to its consensus-based peers.

## Overview

The Archive node serves as the persistence layer for the Offgrid protocol, maintaining a complete historical record of:
- **Account State** - Current balances and positions for all accounts
- **Transactions** - Complete transaction history with timestamps
- **Core App Events** - All events emitted from the runtime (orders, trades, cancellations, etc.)
- **State Snapshots** - Periodic snapshots for fast recovery and state verification

## Features

- **Event Archiving** - Consumes events from runtime via ZMQ and stores them in a database
- **State Management** - Maintains up-to-date account state from transaction history
- **L1 Integration** - Prepares and submits state changes to Layer 1 blockchain
- **Consensus Support** - Can sync with other archive nodes in a consensus network
- **Read Replicas** - Provides read-only access for gateway nodes to query state

## Prerequisites

- **Rust** (latest stable version, recommended 1.70+)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Database** (PostgreSQL recommended, or SQLite for development)
  - **PostgreSQL**: `sudo apt-get install postgresql postgresql-contrib`
  - **SQLite**: Usually included with system
- **ZeroMQ** development libraries (for event consumption)
  - **Ubuntu/Debian**: `sudo apt-get install libzmq3-dev`
  - **macOS**: `brew install zeromq`
  - **Fedora**: `sudo dnf install zeromq-devel`

## Building

### From the workspace root:

```bash
# Build the archive node in release mode
cargo build --release -p offgrid-archive

# Or build from the archive directory
cd crates/archive
cargo build --release
```

### Build artifacts:

The compiled binary will be located at:
```
target/release/offgrid-archive
```

## Configuration

The archive node can be configured using environment variables:

### Database Connection

- `DATABASE_URL` - Database connection string
  - PostgreSQL: `postgresql://user:password@localhost:5432/offgrid_archive`
  - SQLite: `sqlite://./data/archive.db`
  - Default: `sqlite://./data/archive.db`

### Runtime Connection

- `RUNTIME_EVENT_ENDPOINT` - ZMQ endpoint to subscribe to runtime events
  - Default: `tcp://localhost:5555`
- `RUNTIME_ORDER_ENDPOINT` - ZMQ endpoint for order processing (if needed)
  - Default: `tcp://localhost:5556`

### L1 Integration

- `L1_RPC_URL` - Layer 1 blockchain RPC endpoint
  - Default: Not set (L1 integration disabled)
- `L1_PRIVATE_KEY` - Private key for signing L1 transactions
  - Default: Not set (read-only mode)
- `L1_SUBMIT_INTERVAL_SECONDS` - Interval between L1 state submissions
  - Default: `300` (5 minutes)

### Archive Settings

- `ARCHIVE_BATCH_SIZE` - Number of events to batch before writing to database
  - Default: `100`
- `ARCHIVE_FLUSH_INTERVAL_SECONDS` - Maximum time to wait before flushing batch
  - Default: `5` seconds
- `READ_REPLICA_ENABLED` - Enable read replica mode for gateway queries
  - Default: `true`

### Example Configuration

```bash
export DATABASE_URL="postgresql://offgrid:password@localhost:5432/offgrid_archive"
export RUNTIME_EVENT_ENDPOINT="tcp://localhost:5555"
export L1_RPC_URL="https://mainnet.infura.io/v3/YOUR_KEY"
export ARCHIVE_BATCH_SIZE=100
export ARCHIVE_FLUSH_INTERVAL_SECONDS=5
export READ_REPLICA_ENABLED=true
```

## Running

### Basic Usage

```bash
# Run from workspace root
cargo run --release -p offgrid-archive

# Or run from archive directory
cd crates/archive
cargo run --release
```

### With Custom Configuration

```bash
DATABASE_URL="postgresql://user:pass@localhost/archive" \
RUNTIME_EVENT_ENDPOINT="tcp://localhost:5555" \
ARCHIVE_BATCH_SIZE=100 \
cargo run --release -p offgrid-archive
```

### Production Deployment

```bash
# Build release binary
cargo build --release -p offgrid-archive

# Run the binary
./target/release/offgrid-archive
```

### Database Setup

#### PostgreSQL

```bash
# Create database
createdb offgrid_archive

# Run migrations (if using a migration tool)
# diesel migration run
```

#### SQLite

```bash
# Create data directory
mkdir -p ./data

# Database will be created automatically on first run
```

## Architecture

### Event Flow

```
Runtime (ZMQ PUB) → Archive Node (ZMQ SUB) → Event Queue → Batch Processor
                                                                    ↓
                                                            Database Writer
                                                                    ↓
                                                    ┌───────────────┼───────────────┐
                                                    ↓               ↓               ↓
                                            Events Table    Transactions Table  State Table
```

### Thread Model

1. **Main Thread** - Orchestrates all components
2. **Event Consumer Thread** - Subscribes to runtime events via ZMQ
3. **Batch Processor Thread** - Batches events for efficient database writes
4. **Database Writer Thread** - Writes batches to database
5. **State Manager Thread** - Maintains account state from transactions
6. **L1 Submitter Thread** - Submits state changes to Layer 1 (if enabled)
7. **Read Replica Server Thread** - Serves read queries from gateway nodes

### Database Schema

#### Events Table
- `id` - Primary key
- `event_type` - Type of event (OrderPlaced, OrderMatched, etc.)
- `event_data` - JSON serialized event data
- `timestamp` - Event timestamp
- `block_height` - L1 block height (if synced)
- `tx_hash` - L1 transaction hash (if submitted)

#### Transactions Table
- `id` - Primary key
- `tx_hash` - Transaction hash
- `from_account` - Source account
- `to_account` - Destination account
- `amount` - Transaction amount
- `asset_id` - Asset identifier
- `timestamp` - Transaction timestamp
- `block_height` - L1 block height

#### State Table
- `account_id` - Account identifier
- `asset_id` - Asset identifier
- `balance` - Current balance
- `last_updated` - Last update timestamp
- `version` - State version for optimistic locking

## Integration with Runtime

The archive node subscribes to runtime events:

```bash
# Ensure runtime is running and publishing events
# Archive will automatically connect and start consuming events
```

## Integration with Gateway

Gateway nodes connect to archive read replicas to query state:

```bash
# Gateway connects to archive database (read-only)
# Queries account balances, transaction history, etc.
```

## L1 Submission

When configured with L1 credentials, the archive node will:

1. Batch state changes periodically
2. Create L1 transactions with state updates
3. Submit transactions to L1 network
4. Update local records with L1 block height and tx hash

## Monitoring

### Health Checks

- Database connection status
- Event consumption rate
- Batch processing latency
- L1 submission status (if enabled)

### Metrics

- `events_archived_total` - Total events archived
- `events_archived_per_second` - Archive throughput
- `database_write_latency` - Database write performance
- `l1_submissions_total` - Total L1 submissions
- `l1_submission_success_rate` - L1 submission success rate

## Development

### Running Tests

```bash
# Run all tests
cargo test -p offgrid-archive

# Run tests with output
cargo test -p offgrid-archive -- --nocapture
```

### Code Structure

```
crates/archive/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── event_consumer.rs # ZMQ event subscription
│   ├── database/         # Database layer
│   │   ├── models.rs     # Data models
│   │   ├── schema.rs     # Database schema
│   │   └── migrations/   # Database migrations
│   ├── state_manager.rs  # Account state management
│   ├── l1_submitter.rs   # L1 blockchain integration
│   └── read_replica.rs   # Read replica server
└── migrations/           # Database migration files
```

## Troubleshooting

### Database Connection Issues

- Verify database is running: `pg_isready` (PostgreSQL)
- Check connection string format
- Ensure database user has proper permissions
- Check firewall rules for database port

### Event Consumption Issues

- Verify runtime is running and publishing events
- Check ZMQ endpoint configuration
- Test connection: `zmq_sub -t "" tcp://localhost:5555`
- Check network connectivity between runtime and archive

### L1 Submission Failures

- Verify L1 RPC endpoint is accessible
- Check private key format and permissions
- Ensure sufficient balance for gas fees
- Review L1 network status

### Performance Issues

- Increase `ARCHIVE_BATCH_SIZE` for higher throughput
- Adjust `ARCHIVE_FLUSH_INTERVAL_SECONDS` for latency vs throughput tradeoff
- Monitor database connection pool size
- Consider read replicas for query load

## License

[Add your license information here]
