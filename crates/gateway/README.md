# Offgrid Gateway

Offgrid Gateway is the server component that reads records from archive DB read replica and sends user requests to offgrid_runtime. To submit a trading request from gateway to runtime, it must check if both the archive node and runtime node are available.

## Overview

The Gateway serves as the API layer between users and the Offgrid protocol, providing:

- **REST API** - HTTP endpoints for trading operations
- **WebSocket API** - Real-time event streaming to clients
- **State Queries** - Read account state and transaction history from archive
- **Order Submission** - Forward trading requests to runtime
- **Health Monitoring** - Checks availability of runtime and archive nodes

## Features

- **Order Management** - Submit limit orders, market orders, and cancellations
- **Account Queries** - Query balances, positions, and transaction history
- **Real-time Events** - Stream order updates and trade executions via WebSocket
- **Health Checks** - Monitors runtime and archive node availability
- **Request Validation** - Validates orders before forwarding to runtime
- **Rate Limiting** - Protects against abuse and ensures fair access

## Prerequisites

- **Rust** (latest stable version, recommended 1.70+)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **ZeroMQ** development libraries (for runtime communication)
  - **Ubuntu/Debian**: `sudo apt-get install libzmq3-dev`
  - **macOS**: `brew install zeromq`
  - **Fedora**: `sudo dnf install zeromq-devel`
- **Database Client Libraries** (for archive queries)
  - PostgreSQL: `sudo apt-get install libpq-dev`
  - SQLite: Usually included with system

## Building

### From the workspace root:

```bash
# Build the gateway in release mode
cargo build --release -p offgrid-gateway

# Or build from the gateway directory
cd crates/gateway
cargo build --release
```

### Build artifacts:

The compiled binary will be located at:
```
target/release/offgrid-gateway
```

## Configuration

The gateway can be configured using environment variables:

### Server Settings

- `GATEWAY_PORT` - HTTP server port
  - Default: `8080`
- `GATEWAY_HOST` - Server bind address
  - Default: `0.0.0.0`
- `WEBSOCKET_PORT` - WebSocket server port
  - Default: `8081`

### Runtime Connection

- `RUNTIME_ORDER_ENDPOINT` - ZMQ endpoint for sending orders to runtime
  - Default: `tcp://localhost:5556`
- `RUNTIME_EVENT_ENDPOINT` - ZMQ endpoint for subscribing to runtime events
  - Default: `tcp://localhost:5555`
- `RUNTIME_HEALTH_CHECK_INTERVAL_SECONDS` - Interval between runtime health checks
  - Default: `5` seconds

### Archive Connection

- `ARCHIVE_DATABASE_URL` - Archive database connection string (read-only)
  - PostgreSQL: `postgresql://user:password@localhost:5432/offgrid_archive?readonly=true`
  - SQLite: `sqlite://./data/archive.db?mode=ro`
  - Default: `sqlite://./data/archive.db?mode=ro`
- `ARCHIVE_HEALTH_CHECK_INTERVAL_SECONDS` - Interval between archive health checks
  - Default: `5` seconds

### Security

- `API_KEY_REQUIRED` - Require API key for all requests
  - Default: `false`
- `RATE_LIMIT_REQUESTS_PER_MINUTE` - Rate limit per client
  - Default: `1000`
- `MAX_ORDER_SIZE` - Maximum order size in base units
  - Default: `1000000000000` (1 trillion)

### Example Configuration

```bash
export GATEWAY_PORT=8080
export GATEWAY_HOST=0.0.0.0
export WEBSOCKET_PORT=8081
export RUNTIME_ORDER_ENDPOINT="tcp://localhost:5556"
export RUNTIME_EVENT_ENDPOINT="tcp://localhost:5555"
export ARCHIVE_DATABASE_URL="postgresql://readonly:password@localhost:5432/offgrid_archive"
export RATE_LIMIT_REQUESTS_PER_MINUTE=1000
```

## Running

### Basic Usage

```bash
# Run from workspace root
cargo run --release -p offgrid-gateway

# Or run from gateway directory
cd crates/gateway
cargo run --release
```

### With Custom Configuration

```bash
GATEWAY_PORT=8080 \
RUNTIME_ORDER_ENDPOINT="tcp://localhost:5556" \
ARCHIVE_DATABASE_URL="postgresql://user:pass@localhost/archive" \
cargo run --release -p offgrid-gateway
```

### Production Deployment

```bash
# Build release binary
cargo build --release -p offgrid-gateway

# Run the binary
./target/release/offgrid-gateway
```

## API Endpoints

### REST API

#### Health Checks

- `GET /health` - Gateway health status
- `GET /health/runtime` - Runtime node availability
- `GET /health/archive` - Archive node availability

#### Account Queries

- `GET /api/v1/accounts/{account_id}/balance` - Get account balance
- `GET /api/v1/accounts/{account_id}/positions` - Get account positions
- `GET /api/v1/accounts/{account_id}/transactions` - Get transaction history
- `GET /api/v1/accounts/{account_id}/orders` - Get order history

#### Order Management

- `POST /api/v1/orders/limit` - Submit limit order
- `POST /api/v1/orders/market` - Submit market order
- `DELETE /api/v1/orders/{order_id}` - Cancel order
- `GET /api/v1/orders/{order_id}` - Get order status

#### Market Data

- `GET /api/v1/markets/{pair_id}/orderbook` - Get orderbook snapshot
- `GET /api/v1/markets/{pair_id}/trades` - Get recent trades
- `GET /api/v1/markets/{pair_id}/ticker` - Get market ticker

### WebSocket API

- `ws://localhost:8081/events` - Subscribe to real-time events
  - Events: `order_placed`, `order_matched`, `order_cancelled`, `order_filled`, etc.

## Architecture

### Request Flow

```
Client Request → Gateway API → Validation → Health Check
                                              ↓
                                    ┌─────────┴─────────┐
                                    ↓                   ↓
                            Archive (Read)      Runtime (Write)
                                    ↓                   ↓
                            State Query          Order Submission
                                    ↓                   ↓
                            Response ←─────────────────┘
```

### Event Flow

```
Runtime (ZMQ PUB) → Gateway (ZMQ SUB) → Event Router → WebSocket Clients
                                                              ↓
                                                    Filtered Events
```

### Thread Model

1. **Main Thread** - HTTP/WebSocket server
2. **Request Handler Threads** - Process HTTP requests (async/thread pool)
3. **Event Consumer Thread** - Subscribes to runtime events via ZMQ
4. **WebSocket Manager Thread** - Manages WebSocket connections
5. **Health Check Thread** - Periodically checks runtime and archive availability
6. **Rate Limiter Thread** - Tracks and enforces rate limits

## Integration

### With Runtime

The gateway connects to runtime for order submission:

```bash
# Ensure runtime is running
# Gateway will connect to RUNTIME_ORDER_ENDPOINT
# Health check will verify connection
```

### With Archive

The gateway connects to archive for state queries:

```bash
# Ensure archive is running and database is accessible
# Gateway uses read-only connection
# Health check will verify database connectivity
```

## Health Checks

Before processing orders, the gateway verifies:

1. **Runtime Availability** - Can connect to runtime ZMQ endpoint
2. **Archive Availability** - Can query archive database
3. **Service Health** - Internal service status

If either runtime or archive is unavailable, the gateway will:
- Return `503 Service Unavailable` for write operations
- Continue serving read-only queries from cache (if available)
- Log warnings and retry connections

## Security

### API Keys

When `API_KEY_REQUIRED=true`:
- All requests must include `X-API-Key` header
- API keys are validated against configured key store
- Invalid keys return `401 Unauthorized`

### Rate Limiting

- Per-client rate limiting based on IP address
- Configurable requests per minute
- Returns `429 Too Many Requests` when exceeded

### Input Validation

- Order size limits
- Price range validation
- Account ownership verification
- Signature verification (if applicable)

## Monitoring

### Health Endpoints

- `GET /health` - Overall gateway health
- `GET /metrics` - Prometheus metrics (if enabled)

### Metrics

- `requests_total` - Total HTTP requests
- `requests_per_second` - Request throughput
- `order_submissions_total` - Total orders submitted
- `order_submissions_success_rate` - Order submission success rate
- `runtime_health_status` - Runtime availability (0 or 1)
- `archive_health_status` - Archive availability (0 or 1)
- `websocket_connections` - Active WebSocket connections

## Development

### Running Tests

```bash
# Run all tests
cargo test -p offgrid-gateway

# Run tests with output
cargo test -p offgrid-gateway -- --nocapture
```

### Code Structure

```
crates/gateway/
├── src/
│   ├── main.rs           # Entry point and server setup
│   ├── lib.rs            # Library exports
│   ├── api/              # REST API handlers
│   │   ├── health.rs     # Health check endpoints
│   │   ├── accounts.rs   # Account query endpoints
│   │   ├── orders.rs     # Order management endpoints
│   │   └── markets.rs    # Market data endpoints
│   ├── websocket/        # WebSocket server
│   ├── runtime_client.rs # Runtime ZMQ client
│   ├── archive_client.rs # Archive database client
│   ├── health_checker.rs # Health monitoring
│   └── rate_limiter.rs   # Rate limiting logic
└── tests/                # Integration tests
```

## Troubleshooting

### Runtime Connection Issues

- Verify runtime is running: Check runtime logs
- Test ZMQ connection: `zmq_req tcp://localhost:5556`
- Check firewall rules
- Verify `RUNTIME_ORDER_ENDPOINT` configuration

### Archive Connection Issues

- Verify archive database is accessible
- Test database connection: `psql $ARCHIVE_DATABASE_URL`
- Check database user permissions (read-only)
- Verify `ARCHIVE_DATABASE_URL` format

### API Issues

- Check gateway logs for error messages
- Verify port is not in use: `lsof -i :8080`
- Test health endpoint: `curl http://localhost:8080/health`
- Check rate limiting settings

### WebSocket Issues

- Verify WebSocket port is accessible
- Test connection: `wscat -c ws://localhost:8081/events`
- Check firewall rules for WebSocket port
- Review WebSocket connection limits

## Example Usage

### Submit Limit Order

```bash
curl -X POST http://localhost:8080/api/v1/orders/limit \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "pair_id": "BTC/USD",
    "side": "buy",
    "price": 50000,
    "amount": 1000,
    "time_in_force": "GTC"
  }'
```

### Query Account Balance

```bash
curl http://localhost:8080/api/v1/accounts/0x1234/balance?asset_id=BTC
```

### Subscribe to Events (WebSocket)

```javascript
const ws = new WebSocket('ws://localhost:8081/events');
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data);
};
```

## License

[Add your license information here]
