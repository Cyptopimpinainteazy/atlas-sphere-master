# Atlas Sphere Indexer

Blockchain indexer for the Atlas Sphere dual-VM network. Subscribes to finalized blocks and stores data in PostgreSQL for querying.

## Features

- **Block indexing**: Indexes blocks, extrinsics, and events
- **Comit tracking**: Special handling for Atlas Kernel Comit transactions
- **Account tracking**: Monitors account balances and authorization status
- **PostgreSQL storage**: Persistent storage with efficient indexes
- **Prometheus metrics**: Monitoring and alerting support
- **Health endpoints**: HTTP health checks for orchestration

## Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Running Atlas Sphere node (WebSocket RPC endpoint)

## Quick Start

### 1. Set up PostgreSQL

```bash
# Create database
createdb atlas_indexer

# Or with Docker
docker run -d --name atlas-postgres \
  -e POSTGRES_USER=indexer \
  -e POSTGRES_PASSWORD=indexer \
  -e POSTGRES_DB=atlas_indexer \
  -p 5432:5432 \
  postgres:14
```

### 2. Configure

Copy and customize the config file:

```bash
cp indexer.toml indexer.local.toml
# Edit indexer.local.toml with your settings
```

Or use environment variables:
```bash
export DATABASE_URL="postgres://indexer:indexer@localhost:5432/atlas_indexer"
export NODE_URL="ws://127.0.0.1:9944"
```

### 3. Run

```bash
# Build
cargo build --release -p atlas-indexer

# Run with config file
./target/release/atlas-indexer --config indexer.local.toml

# Or with environment variables
./target/release/atlas-indexer
```

## Configuration

The indexer can be configured via:
1. TOML config file (`--config path/to/config.toml`)
2. Environment variables (prefixed with uppercase, e.g., `NODE_URL`)
3. Command line arguments

### Config File Options

```toml
[node]
url = "ws://127.0.0.1:9944"   # Node WebSocket URL
timeout_ms = 30000            # Connection timeout
max_reconnects = 0            # 0 = infinite reconnects
reconnect_delay_secs = 5      # Delay between reconnection attempts

[database]
url = "postgres://user:pass@localhost:5432/atlas_indexer"
max_connections = 10
min_connections = 1
acquire_timeout_secs = 30

[indexer]
start_block = 0               # Starting block (null = resume or genesis)
batch_size = 100              # Blocks per batch during catchup
max_concurrent = 4            # Concurrent block processing
poll_interval_ms = 1000       # Poll interval when caught up
store_raw = false             # Store raw extrinsic bytes
index_comits = true           # Index Comit transactions

[metrics]
enabled = true
host = "127.0.0.1"
port = 9615
```

## HTTP Endpoints

| Endpoint   | Description                         |
| ---------- | ----------------------------------- |
| `/health`  | Health check (always returns 200)   |
| `/ready`   | Readiness check (200 if connected)  |
| `/metrics` | Prometheus metrics                  |
| `/status`  | JSON status (block heights, errors) |

## Database Schema

The indexer creates the following tables:

- `indexer_state` - Key-value state (last indexed block, etc.)
- `blocks` - Block headers and metadata
- `extrinsics` - All extrinsics with pallet/call info
- `events` - All events with pallet/variant info
- `comit_transactions` - Atlas Kernel Comit transactions
- `accounts` - Account balances and metadata
- `asset_balances` - Per-asset balances
- `evm_logs` - EVM contract logs
- `svm_instructions` - SVM program instructions

## Metrics

Prometheus metrics exposed at `/metrics`:

- `indexer_blocks_indexed_total` - Total blocks processed
- `indexer_latest_block` - Most recent indexed block
- `indexer_block_processing_ms` - Block processing time histogram
- `indexer_errors_total` - Total errors encountered

## Development

```bash
# Run tests
cargo test -p atlas-indexer

# Check compilation
cargo check -p atlas-indexer

# Run with debug logging
RUST_LOG=atlas_indexer=debug cargo run -p atlas-indexer
```

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│  Atlas Node     │────▶│  Indexer        │
│  (WebSocket)    │     │  (subxt client) │
└─────────────────┘     └───────┬─────────┘
                                │
                                ▼
                        ┌───────────────┐
                        │  PostgreSQL   │
                        │  (sqlx)       │
                        └───────────────┘
                                │
                                ▼
                        ┌───────────────┐
                        │  HTTP Server  │
                        │  (axum)       │
                        └───────────────┘
                                │
                    ┌───────────┼───────────┐
                    ▼           ▼           ▼
               /health     /metrics    /status
```

## License

MIT OR Apache-2.0
