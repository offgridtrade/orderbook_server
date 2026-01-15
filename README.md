# Offgrid Matching Engine

This repository contains the matching engine and supporting services for the Offgrid protocol.
It is organized as a Rust workspace with multiple components that work together to process
orders, emit events, persist state, and expose APIs to clients.

## Components

- **primitives**: Core data structures and matching logic (orderbook L1/L2/L3, matching engine, events).
- **runtime**: In-memory application layer; orchestrates event bus, networking, metrics, and snapshots.
- **archive**: Persistence layer; stores runtime events and transactions, maintains account state.
- **gateway**: API layer; reads from archive and forwards requests to runtime.

## How Components Fit Together

```
                +--------------------+
                |      Gateway       |
                |  REST/WebSocket    |
                +---------+----------+
                          |
                          | order requests (ZMQ)
                          v
                +---------+----------+
                |      Runtime       |
                |  Matching Engine   |
                |  Event Bus         |
                +---------+----------+
                          |
                          | events (ZMQ PUB)
                          v
                +---------+----------+
                |      Archive       |
                |  DB / State Store  |
                +---------+----------+
                          ^
                          |
                read queries (DB)
                          |
                +---------+----------+
                |      Gateway       |
                +--------------------+
```

## Repository Layout

```
crates/
  primitives/   # core orderbook + events
  runtime/      # in-memory runtime and networking
  archive/      # persistence / state archiving
  gateway/      # API server and clients
```