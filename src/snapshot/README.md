# Snapshot Requirements

A RocksDB snapshot must capture the full state required to resume the orderbook service. Persist and restore the following components:

- `OrderStorage` for the active pair, including per-price linked lists of order IDs.
- `PriceLinkedList` so price levels and their ordering remain intact.
- Level 1 (`L1`) metadata for the pair (best bid/ask and any cached aggregates).

When loading a snapshot, rebuild each structure in the same order they were persisted to guarantee consistency across restarts.
